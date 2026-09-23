use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Recipes {
    baselines: BTreeMap<String, Baseline>,
    cases: Vec<RecipeCase>,
}

#[derive(Debug, Deserialize)]
struct Baseline {
    source: String,
    source_sha256: String,
    #[serde(default)]
    mutations: Vec<Mutation>,
}

#[derive(Debug, Deserialize)]
struct RecipeCase {
    id: String,
    family: String,
    baseline: String,
    #[serde(default)]
    mutations: Vec<Mutation>,
    expected_schema_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Mutation {
    op: String,
    path: String,
    #[serde(default, deserialize_with = "present_value")]
    value: Option<Value>,
}

fn present_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a repository parent")
        .to_path_buf()
}

fn corpus_root() -> PathBuf {
    repository_root().join("xtask/tests/corpus/record-schema")
}

fn recipes() -> Recipes {
    serde_json::from_slice(
        &std::fs::read(corpus_root().join("CASES.json")).expect("record-schema cases"),
    )
    .expect("record-schema cases JSON")
}

fn decode_pointer(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    assert!(path.starts_with('/'), "invalid JSON Pointer {path:?}");
    path[1..]
        .split('/')
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn index(segment: &str, path: &str) -> usize {
    segment
        .parse()
        .unwrap_or_else(|_| panic!("invalid array index {segment:?} in {path}"))
}

fn child_mut<'a>(value: &'a mut Value, segment: &str, path: &str) -> &'a mut Value {
    match value {
        Value::Object(object) => object
            .get_mut(segment)
            .unwrap_or_else(|| panic!("missing JSON Pointer parent {path}")),
        Value::Array(array) => {
            let position = index(segment, path);
            array
                .get_mut(position)
                .unwrap_or_else(|| panic!("missing JSON Pointer parent {path}"))
        }
        _ => panic!("non-container JSON Pointer parent for {path}"),
    }
}

fn mutation_value(mutation: &Mutation) -> Value {
    mutation
        .value
        .clone()
        .unwrap_or_else(|| panic!("{} requires a value", mutation.op))
}

fn apply_mutation(value: &mut Value, mutation: &Mutation) {
    let parts = decode_pointer(&mutation.path);
    if parts.is_empty() {
        match mutation.op.as_str() {
            "set" | "replace" => *value = mutation_value(mutation),
            "remove" => panic!("root removal is not a defined recipe"),
            op => panic!("unknown mutation operation {op}"),
        }
        return;
    }

    let last = parts.last().expect("non-empty pointer").clone();
    let parent = parts[..parts.len() - 1]
        .iter()
        .fold(value, |current, segment| {
            child_mut(current, segment, &mutation.path)
        });

    match parent {
        Value::Object(object) => match mutation.op.as_str() {
            "set" => {
                object.insert(last, mutation_value(mutation));
            }
            "replace" => {
                assert!(
                    object.contains_key(&last),
                    "replace target is absent: {}",
                    mutation.path
                );
                object.insert(last, mutation_value(mutation));
            }
            "remove" => {
                assert!(
                    object.remove(&last).is_some(),
                    "remove target is absent: {}",
                    mutation.path
                );
            }
            op => panic!("unknown mutation operation {op}"),
        },
        Value::Array(array) => {
            let position = index(&last, &mutation.path);
            match mutation.op.as_str() {
                "set" | "replace" => {
                    assert!(
                        position < array.len(),
                        "array replacement target is absent: {}",
                        mutation.path
                    );
                    array[position] = mutation_value(mutation);
                }
                "remove" => {
                    assert!(
                        position < array.len(),
                        "array removal target is absent: {}",
                        mutation.path
                    );
                    array.remove(position);
                }
                op => panic!("unknown mutation operation {op}"),
            }
        }
        _ => panic!("non-container JSON Pointer parent for {}", mutation.path),
    }
}

fn parse_source(path: &Path) -> Value {
    let bytes = std::fs::read(path).unwrap_or_else(|error| {
        panic!("reading source {}: {error}", path.display());
    });
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        serde_json::from_slice(&bytes).unwrap_or_else(|error| {
            panic!("parsing JSON source {}: {error}", path.display());
        })
    } else {
        let text = std::str::from_utf8(&bytes).expect("TOML source is UTF-8");
        let toml_value: toml::Value = toml::from_str(text).unwrap_or_else(|error| {
            panic!("parsing TOML source {}: {error}", path.display());
        });
        serde_json::to_value(toml_value).expect("TOML converts to JSON")
    }
}

fn baseline_value(baseline: &Baseline) -> Value {
    let source_path = corpus_root().join(&baseline.source);
    let source_bytes = std::fs::read(&source_path).unwrap_or_else(|error| {
        panic!("reading source {}: {error}", source_path.display());
    });
    assert_eq!(
        xtask::util::sha256_hex(&source_bytes),
        baseline.source_sha256,
        "source hash changed: {}",
        baseline.source
    );

    let mut value = parse_source(&source_path);
    for mutation in &baseline.mutations {
        apply_mutation(&mut value, mutation);
    }
    value
}

#[test]
fn every_registered_record_schema_recipe_matches_exactly() {
    assert_eq!(xtask::record_schema::configuration_error(), None);

    let recipes = recipes();
    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(corpus_root().join("registration.toml"))
            .expect("record-schema registration"),
    )
    .expect("record-schema registration TOML");
    assert_eq!(registration["case_count"].as_integer(), Some(1497));
    assert_eq!(recipes.cases.len(), 1497);

    let baselines: BTreeMap<String, Value> = recipes
        .baselines
        .iter()
        .map(|(name, baseline)| (name.clone(), baseline_value(baseline)))
        .collect();

    let mut counts = BTreeMap::<String, usize>::new();
    let mut failures = Vec::new();
    for case in &recipes.cases {
        *counts.entry(case.family.clone()).or_default() += 1;
        let mut value = baselines
            .get(&case.baseline)
            .unwrap_or_else(|| panic!("unknown baseline {}", case.baseline))
            .clone();
        for mutation in &case.mutations {
            apply_mutation(&mut value, mutation);
        }

        let actual: BTreeSet<String> = xtask::record_schema::validate(&value, &case.family)
            .into_iter()
            .map(|issue| issue.code.to_owned())
            .collect();
        let expected: BTreeSet<String> = case.expected_schema_codes.iter().cloned().collect();
        if actual != expected {
            let details = xtask::record_schema::validate(&value, &case.family)
                .into_iter()
                .map(|issue| format!("{} {} {}", issue.code, issue.path, issue.message))
                .collect::<Vec<_>>();
            failures.push(format!(
                "{} ({}) expected {expected:?}, got {actual:?}: {}",
                case.id,
                case.family,
                details.join("; ")
            ));
        }
    }

    let registered_counts: BTreeMap<String, usize> = registration["case_counts_by_family"]
        .as_table()
        .expect("case_counts_by_family table")
        .iter()
        .map(|(family, count)| {
            (
                family.clone(),
                usize::try_from(count.as_integer().expect("family count integer"))
                    .expect("family count fits usize"),
            )
        })
        .collect();
    assert_eq!(counts, registered_counts);
    for (family, count) in &counts {
        println!("record-schema {family}: {count}");
    }
    println!("record-schema total: {}", recipes.cases.len());

    assert!(
        failures.is_empty(),
        "{} of {} record-schema recipes differed:\n{}",
        failures.len(),
        recipes.cases.len(),
        failures.join("\n")
    );
}
