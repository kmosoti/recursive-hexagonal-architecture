use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

const SUPPLEMENT_CONTENT_SHA256: &str =
    "e271498e5619751e3fd8d50eeee26728a7154d3b9cb451f9c75cffa37913b183";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    id: String,
    baseline: String,
    mutations: Vec<Mutation>,
    expected_schema_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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

fn supplement_root() -> PathBuf {
    repository_root().join("xtask/tests/corpus/markdown-schema-supplement")
}

fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(path).is_relative()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn payload_files(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<String, String>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(directory)
            .expect("supplement directory")
            .map(|entry| entry.expect("supplement entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let metadata = std::fs::symlink_metadata(&path).expect("supplement metadata");
            assert!(
                !metadata.file_type().is_symlink(),
                "supplement contains symlink: {}",
                path.display()
            );
            if metadata.is_dir() {
                walk(root, &path, files);
            } else {
                assert!(
                    metadata.is_file(),
                    "supplement contains non-file: {}",
                    path.display()
                );
                let relative = path
                    .strip_prefix(root)
                    .expect("supplement relative path")
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative != "registration.toml" && relative != "SHA256SUMS" {
                    let bytes = std::fs::read(&path).expect("supplement payload");
                    assert!(
                        files
                            .insert(relative.clone(), xtask::util::sha256_hex(&bytes))
                            .is_none(),
                        "duplicate supplement path: {relative}"
                    );
                }
            }
        }
    }

    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn verify_frozen_supplement(root: &Path) -> toml::Value {
    let sums_path = root.join("SHA256SUMS");
    let sums = std::fs::read(&sums_path).expect("supplement SHA256SUMS");
    assert_eq!(
        xtask::util::sha256_hex(&sums),
        SUPPLEMENT_CONTENT_SHA256,
        "supplement SHA256SUMS content changed"
    );

    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(root.join("registration.toml")).expect("supplement registration"),
    )
    .expect("supplement registration TOML");
    assert_eq!(
        registration["content_sha256"].as_str(),
        Some(SUPPLEMENT_CONTENT_SHA256)
    );
    assert_eq!(registration["family"].as_str(), Some("markdown_corpus"));
    assert_eq!(registration["after_data"].as_bool(), Some(true));
    assert_eq!(registration["recipes"]["count"].as_integer(), Some(16));

    let text = std::str::from_utf8(&sums).expect("supplement SHA256SUMS UTF-8");
    assert!(
        text.ends_with('\n'),
        "supplement SHA256SUMS must end with LF"
    );
    let mut listed = BTreeMap::new();
    let mut previous = None;
    for line in text[..text.len() - 1].split('\n') {
        assert!(
            line.len() >= 66 && &line[64..66] == "  ",
            "malformed SHA256SUMS line"
        );
        let digest = &line[..64];
        let relative = &line[66..];
        assert!(lower_sha256(digest), "invalid SHA256SUMS digest");
        assert!(safe_relative_path(relative), "unsafe SHA256SUMS path");
        if let Some(old) = previous {
            assert!(old < relative, "SHA256SUMS paths are not strictly sorted");
        }
        previous = Some(relative);
        let path = root.join(relative);
        let metadata = std::fs::symlink_metadata(&path).expect("listed supplement payload");
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        let bytes = std::fs::read(path).expect("listed supplement payload bytes");
        assert_eq!(xtask::util::sha256_hex(&bytes), digest);
        assert!(
            listed
                .insert(relative.to_owned(), digest.to_owned())
                .is_none(),
            "duplicate SHA256SUMS path"
        );
    }
    assert_eq!(listed, payload_files(root));

    for source in registration["sources"]
        .as_array()
        .expect("registered sources")
    {
        let relative = source["path"].as_str().expect("source path");
        let digest = source["sha256"].as_str().expect("source digest");
        assert!(safe_relative_path(relative));
        assert!(lower_sha256(digest));
        let path = repository_root().join(relative);
        let metadata = std::fs::symlink_metadata(&path).expect("registered source");
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        assert_eq!(
            xtask::util::sha256_hex(&std::fs::read(path).expect("registered source bytes")),
            digest
        );
    }

    registration
}

fn decode_pointer(path: &str) -> Vec<String> {
    assert!(
        path.starts_with('/') && path.len() > 1,
        "invalid JSON Pointer {path:?}"
    );
    path[1..]
        .split('/')
        .map(|raw| {
            let mut decoded = String::new();
            let mut characters = raw.chars();
            while let Some(character) = characters.next() {
                if character != '~' {
                    decoded.push(character);
                } else {
                    match characters.next() {
                        Some('0') => decoded.push('~'),
                        Some('1') => decoded.push('/'),
                        other => panic!("invalid JSON Pointer escape {other:?} in {path:?}"),
                    }
                }
            }
            decoded
        })
        .collect()
}

fn array_index(segment: &str, path: &str) -> usize {
    segment
        .parse()
        .unwrap_or_else(|_| panic!("invalid array index {segment:?} in {path}"))
}

fn child_mut<'a>(value: &'a mut Value, segment: &str, path: &str) -> &'a mut Value {
    match value {
        Value::Object(object) => object
            .get_mut(segment)
            .unwrap_or_else(|| panic!("missing JSON Pointer parent {path}")),
        Value::Array(array) => array
            .get_mut(array_index(segment, path))
            .unwrap_or_else(|| panic!("missing JSON Pointer parent {path}")),
        _ => panic!("non-container JSON Pointer parent for {path}"),
    }
}

fn mutation_value(mutation: &Mutation) -> Value {
    mutation
        .value
        .clone()
        .unwrap_or_else(|| panic!("{} requires a present value", mutation.op))
}

fn apply_mutation(value: &mut Value, mutation: &Mutation) {
    assert!(
        mutation.op == "set" || mutation.op == "remove",
        "unsupported mutation operation {}",
        mutation.op
    );
    if mutation.op == "remove" {
        assert!(mutation.value.is_none(), "remove mutation must omit value");
    }

    let parts = decode_pointer(&mutation.path);
    let last = parts.last().expect("non-empty JSON Pointer").clone();
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
            "remove" => {
                assert!(
                    object.remove(&last).is_some(),
                    "remove target is absent: {}",
                    mutation.path
                );
            }
            _ => unreachable!(),
        },
        Value::Array(array) => {
            let position = array_index(&last, &mutation.path);
            assert!(
                position < array.len(),
                "array target is absent: {}",
                mutation.path
            );
            match mutation.op.as_str() {
                "set" => array[position] = mutation_value(mutation),
                "remove" => {
                    array.remove(position);
                }
                _ => unreachable!(),
            }
        }
        _ => panic!("non-container JSON Pointer parent for {}", mutation.path),
    }
}

fn baseline_value(root: &Path, baseline: &str) -> Value {
    assert!(matches!(baseline, "old.json" | "new.json"));
    serde_json::from_slice(&std::fs::read(root.join(baseline)).expect("supplement baseline"))
        .expect("supplement baseline JSON")
}

#[test]
fn frozen_markdown_shape_supplement_matches_all_registered_recipes() {
    let root = supplement_root();
    let registration = verify_frozen_supplement(&root);
    let recipes: Vec<Recipe> =
        serde_json::from_slice(&std::fs::read(root.join("CASES.json")).expect("recipes"))
            .expect("recipes JSON");
    assert_eq!(recipes.len(), 16);
    assert_eq!(
        registration["recipes"]["count"].as_integer(),
        Some(i64::try_from(recipes.len()).expect("recipe count"))
    );

    let mut accepted = 0;
    let mut rejected = 0;
    for recipe in recipes {
        let mut value = baseline_value(&root, &recipe.baseline);
        for mutation in &recipe.mutations {
            apply_mutation(&mut value, mutation);
        }

        let actual: BTreeSet<String> = xtask::record_schema::validate(&value, "markdown_corpus")
            .into_iter()
            .map(|issue| issue.code.to_owned())
            .collect();
        let expected_codes = recipe.expected_schema_codes;
        let mut sorted_expected = expected_codes.clone();
        sorted_expected.sort();
        sorted_expected.dedup();
        assert_eq!(
            expected_codes, sorted_expected,
            "{} has duplicate or unsorted expected codes",
            recipe.id
        );
        let expected: BTreeSet<String> = expected_codes.into_iter().collect();
        assert_eq!(
            actual, expected,
            "{} expected {expected:?}, got {actual:?}",
            recipe.id
        );
        if expected.is_empty() {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    println!(
        "markdown-schema supplement recipes: total=16 accepted={accepted} rejected={rejected}"
    );
}

#[test]
fn schema_codegen_check_matches_projected_schemas() {
    let output = Command::new("python3")
        .args(["xtask/schema_codegen.py", "--check"])
        .current_dir(repository_root())
        .output()
        .expect("spawn schema codegen");
    assert!(
        output.status.success(),
        "schema codegen --check failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
