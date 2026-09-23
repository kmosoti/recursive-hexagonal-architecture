use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use xtask::corpus::{Expected, Level, MANIFEST_PATH, Manifest, grade};
use xtask::modules::extract::{Edge, Extracted};
use xtask::modules::rules;

const HEADLINE_IDS: [&str; 25] = [
    "M01", "M02", "M03", "M04", "M05", "M06", "M07", "M08", "M09", "M10", "M11", "M12", "M13",
    "M14", "M15", "M16", "M17", "M18", "M19", "M20", "M21", "L-M02", "EM-M01", "EM-M02", "EM-M03",
];

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rha-module-rules-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a workspace parent")
        .to_path_buf()
}

fn manifest() -> Manifest {
    let path = root().join(MANIFEST_PATH);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("reading {}: {error}", path.display());
    });
    Manifest::parse(&text).unwrap_or_else(|error| {
        panic!("parsing {}: {error}", path.display());
    })
}

fn fixture_identity(crate_root: &Path) -> (String, String) {
    let cargo: toml::Value =
        toml::from_str(&fs::read_to_string(crate_root.join("Cargo.toml")).expect("Cargo.toml"))
            .expect("fixture Cargo.toml");
    let package = cargo
        .get("package")
        .and_then(toml::Value::as_table)
        .expect("package table");
    let name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .expect("fixture package name")
        .to_owned();
    let edition = package
        .get("edition")
        .and_then(toml::Value::as_str)
        .expect("fixture edition")
        .to_owned();
    (name, edition)
}

fn module_report(checked: rules::Checked) -> Value {
    let errors = checked
        .findings
        .iter()
        .filter(|finding| finding["severity"] == "error")
        .count();
    let warnings = checked
        .findings
        .iter()
        .filter(|finding| finding["severity"] == "warning")
        .count();
    json!({
        "schema_version": 1,
        "findings": checked.findings,
        "test_edges": checked.test_edges,
        "module_edges": checked.module_edges,
        "limitations": checked.limitations,
        "summary": {
            "errors": errors,
            "warnings": warnings,
            "outcome": if errors == 0 { "passed" } else { "failed" },
            "error_class": Value::Null,
        },
    })
}

#[test]
fn every_authored_headline_fixture_is_graded_against_the_registered_case() {
    let manifest = manifest();
    let fixtures = root().join("xtask/tests/corpus/module/headline");
    let mut failures = Vec::new();
    let mut passed = 0usize;

    assert_eq!(HEADLINE_IDS.len(), 25);
    assert_eq!(
        manifest
            .cases
            .iter()
            .filter(|case| case.level == Level::Module
                && case.id != "L-M01"
                && case.expected != Expected::Reference)
            .count(),
        25
    );

    for id in HEADLINE_IDS {
        let Some(case) = manifest.cases.iter().find(|case| case.id == id) else {
            failures.push(format!("{id}: absent from manifest"));
            continue;
        };
        let crate_root = fixtures.join(id);
        let (crate_name, edition) = fixture_identity(&crate_root);
        let externals = BTreeSet::new();
        let extracted = match xtask::modules::extract::extract(
            &crate_root,
            &crate_root.join("src/lib.rs"),
            &crate_name,
            &edition,
            &externals,
        ) {
            Ok(extracted) => extracted,
            Err(error) => {
                failures.push(format!("{id}: extraction failed: {error}"));
                continue;
            }
        };
        let checked = match rules::check(
            &crate_root,
            &crate_name,
            &crate_root.join("rha-modules.toml"),
            &extracted,
        ) {
            Ok(checked) => checked,
            Err(error) => {
                failures.push(format!("{id}: rules failed: {error}"));
                continue;
            }
        };
        let report = module_report(checked);
        let exit = Some(if report["summary"]["errors"].as_u64().unwrap_or(0) > 0 {
            1
        } else {
            0
        });
        let grade = grade::architecture(case, exit, &report);
        if !grade.passed {
            failures.push(format!(
                "{id}: grade failed: {} report={}",
                grade.reasons.join("; "),
                report
            ));
        } else {
            passed += 1;
        }

        if id == "EM-M03" {
            let source = format!("{crate_name}::constraints");
            let ordering = format!("{crate_name}::ordering");
            let has_visible_ordering = report["module_edges"]
                .as_array()
                .expect("module_edges array")
                .iter()
                .any(|edge| {
                    edge["source"] == source
                        && edge["target"] == ordering
                        && edge["test_only"] == false
                });
            if !has_visible_ordering {
                failures
                    .push("EM-M03: missing visible constraints-to-ordering raw edge".to_owned());
            }
            let has_constraints_to_model = report["module_edges"]
                .as_array()
                .expect("module_edges array")
                .iter()
                .any(|edge| {
                    edge["source"] == source
                        && edge["target"].as_str().is_some_and(|target| {
                            target.starts_with(&format!("{crate_name}::model"))
                        })
                });
            if has_constraints_to_model {
                failures
                    .push("EM-M03: unexpected implicit constraints-to-model raw edge".to_owned());
            }
        }
    }

    eprintln!(
        "module headline grading: {passed}/{} passed",
        HEADLINE_IDS.len()
    );
    assert!(
        failures.is_empty(),
        "headline mismatches:\n{}",
        failures.join("\n")
    );
}

fn materialize_files(root: &Path, files: &serde_json::Map<String, Value>) {
    for (relative, contents) in files {
        let relative_path = Path::new(relative);
        assert!(
            !relative_path.is_absolute()
                && relative_path
                    .components()
                    .all(|component| matches!(component, Component::Normal(_))),
            "unsafe registered relative path: {relative}"
        );
        let contents = contents
            .as_str()
            .unwrap_or_else(|| panic!("registered file {relative} is not a string"));
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().expect("materialized file parent"))
            .expect("materialized parent");
        fs::write(path, contents).expect("materialized file");
    }
}

fn component_paths(rules_path: &Path) -> BTreeMap<String, String> {
    let value: toml::Value =
        toml::from_str(&fs::read_to_string(rules_path).expect("random rules")).expect("rules TOML");
    value
        .get("components")
        .and_then(toml::Value::as_table)
        .expect("components table")
        .iter()
        .map(|(alias, path)| {
            (
                alias.clone(),
                path.as_str().expect("component path string").to_owned(),
            )
        })
        .collect()
}

fn component_owner(path: &str, components: &BTreeMap<String, String>) -> Option<String> {
    components
        .iter()
        .filter(|(_, component)| {
            path == component.as_str()
                || path
                    .strip_prefix(component.as_str())
                    .is_some_and(|suffix| suffix.starts_with("::"))
        })
        .max_by_key(|(_, component)| component.len())
        .map(|(alias, _)| alias.clone())
}

fn projected_component_edges(
    module_edges: &[Value],
    components: &BTreeMap<String, String>,
) -> Vec<Value> {
    let mut edges = module_edges
        .iter()
        .filter(|edge| edge["test_only"] == false)
        .filter_map(|edge| {
            let source = edge["source"].as_str()?;
            let target = edge["target"].as_str()?;
            let source = component_owner(source, components)?;
            let target = component_owner(target, components)?;
            (source != target).then(|| {
                json!({
                    "extraction": edge["extraction"],
                    "source": source,
                    "target": target,
                })
            })
        })
        .collect::<Vec<_>>();
    edges.sort_by_key(Value::to_string);
    edges.dedup();
    edges
}

fn registered_projection(actual: &Value, schema: &Value) -> Option<Value> {
    match schema {
        Value::Object(schema_object) => {
            let actual_object = actual.as_object()?;
            let mut projected = serde_json::Map::new();
            for (key, expected) in schema_object {
                projected.insert(
                    key.clone(),
                    registered_projection(actual_object.get(key)?, expected)?,
                );
            }
            Some(Value::Object(projected))
        }
        Value::Array(schema_array) => {
            let actual_array = actual.as_array()?;
            if actual_array.len() != schema_array.len() {
                return None;
            }
            Some(Value::Array(
                actual_array
                    .iter()
                    .zip(schema_array)
                    .map(|(actual, expected)| registered_projection(actual, expected))
                    .collect::<Option<Vec<_>>>()?,
            ))
        }
        _ if actual == schema => Some(actual.clone()),
        _ => None,
    }
}

fn assert_registered_multiset(
    label: &str,
    expected: &[Value],
    actual: &[Value],
    failures: &mut Vec<String>,
) {
    if expected.len() != actual.len() {
        failures.push(format!(
            "{label}: registered cardinality {} != observed {}",
            expected.len(),
            actual.len()
        ));
        return;
    }

    let mut used = vec![false; actual.len()];
    for expected_value in expected {
        let found = actual.iter().enumerate().position(|(index, actual_value)| {
            !used[index]
                && registered_projection(actual_value, expected_value)
                    .is_some_and(|projected| projected == *expected_value)
        });
        match found {
            Some(index) => used[index] = true,
            None => failures.push(format!(
                "{label}: no observed diagnostic matched {expected_value}"
            )),
        }
    }
}

#[test]
fn registered_random_rule_expectations_are_checked_without_rebuilding_them() {
    let base = root().join("xtask/tests/corpus/module");
    let random_dir = base.join("random");
    let expectations: Value = serde_json::from_str(
        &fs::read_to_string(base.join("random-rule-expectations.json"))
            .expect("random-rule-expectations.json"),
    )
    .expect("random-rule-expectations.json JSON");
    let expectation_object = expectations.as_object().expect("expectation object");

    let mut ids = fs::read_dir(&random_dir)
        .expect("random corpus directory")
        .map(|entry| entry.expect("random corpus entry").file_name())
        .filter_map(|name| name.to_str().map(str::to_owned))
        .filter_map(|name| name.strip_suffix(".json").map(str::to_owned))
        .filter(|id| !id.ends_with(".expected"))
        .collect::<Vec<_>>();
    ids.sort();

    let registration: toml::Value =
        toml::from_str(&fs::read_to_string(base.join("registration.toml")).expect("registration"))
            .expect("registration TOML");
    let registered_count = registration
        .get("counts")
        .and_then(toml::Value::as_table)
        .and_then(|counts| counts.get("random_cases"))
        .and_then(toml::Value::as_integer)
        .expect("registered random case count");
    assert_eq!(registered_count, ids.len() as i64);
    assert_eq!(ids.len(), expectation_object.len());

    let scratch = TempDir::new("random-oracle");
    let mut failures = Vec::new();
    let mut observed_edges = 0usize;
    let mut observed_findings = 0usize;

    for id in &ids {
        let source_map: Value = serde_json::from_str(
            &fs::read_to_string(random_dir.join(format!("{id}.json"))).expect("random source map"),
        )
        .expect("random source map JSON");
        let source_map_object = source_map.as_object().expect("source map object");
        let crate_name = source_map_object["crate_name"]
            .as_str()
            .expect("random crate name");
        let edition = source_map_object["edition"]
            .as_str()
            .expect("random edition");
        let case_root = scratch.path().join(id);
        fs::create_dir_all(&case_root).expect("random case directory");
        materialize_files(
            &case_root,
            source_map_object["files"]
                .as_object()
                .expect("random files"),
        );

        let extracted = match xtask::modules::extract::extract(
            &case_root,
            &case_root.join("src/lib.rs"),
            crate_name,
            edition,
            &BTreeSet::new(),
        ) {
            Ok(extracted) => extracted,
            Err(error) => {
                failures.push(format!("{id}: extraction failed: {error}"));
                continue;
            }
        };
        if !extracted.limitations.is_empty() {
            failures.push(format!(
                "{id}: unexpected extraction limitations: {:?}",
                extracted.limitations
            ));
        }
        let checked = match rules::check(
            &case_root,
            crate_name,
            &case_root.join("rha-modules.toml"),
            &extracted,
        ) {
            Ok(checked) => checked,
            Err(error) => {
                failures.push(format!("{id}: rules failed: {error}"));
                continue;
            }
        };
        observed_edges += checked.module_edges.len();
        observed_findings += checked.findings.len();
        let expected = expectation_object
            .get(id)
            .unwrap_or_else(|| panic!("{id}: absent from registered expectations"));
        let expected_edges = expected["component_edges"]
            .as_array()
            .expect("registered component_edges");
        let expected_findings = expected["violations"]
            .as_array()
            .expect("registered violations");
        let components = component_paths(&case_root.join("rha-modules.toml"));
        let actual_edges = projected_component_edges(&checked.module_edges, &components);
        // The independent oracle uses the concrete crate name; public
        // diagnostics use Rust's `crate::` spelling. Normalize this spelling
        // only, preserving every registered diagnostic fact and cardinality.
        let actual_findings = checked
            .findings
            .into_iter()
            .map(|mut finding| {
                if let Some(tail) = finding["to"]
                    .as_str()
                    .and_then(|to| to.strip_prefix("crate::"))
                {
                    finding["to"] = json!(format!("{crate_name}::{tail}"));
                }
                finding
            })
            .collect::<Vec<_>>();
        assert_registered_multiset(
            &format!("{id} component_edges"),
            expected_edges,
            &actual_edges,
            &mut failures,
        );
        assert_registered_multiset(
            &format!("{id} violations"),
            expected_findings,
            &actual_findings,
            &mut failures,
        );
    }

    eprintln!(
        "random module oracle: {} cases, {observed_edges} raw edges, \
         {observed_findings} findings, {} mismatches",
        ids.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "random rule mismatches:\n{}",
        failures.join("\n")
    );
}

fn synthetic_extracted(crate_name: &str, modules: &[&str], edges: Vec<Edge>) -> Extracted {
    let mut module_paths = BTreeMap::new();
    module_paths.insert(
        crate_name.to_owned(),
        PathBuf::from(format!("/synthetic/{crate_name}/src/lib.rs")),
    );
    for module in modules {
        module_paths.insert(
            format!("{crate_name}::{module}"),
            PathBuf::from(format!("/synthetic/{crate_name}/src/{module}.rs")),
        );
    }
    Extracted {
        modules: module_paths,
        edges,
        limitations: Vec::new(),
    }
}

fn synthetic_edge(source: &str, target: &str, extraction: &str) -> Edge {
    Edge {
        source: source.to_owned(),
        target: target.to_owned(),
        test_only: false,
        extraction: extraction.to_owned(),
    }
}

fn write_rules(temp: &TempDir, text: &str) -> PathBuf {
    let path = temp.path().join("rha-modules.toml");
    fs::write(&path, text).expect("synthetic module rules");
    path
}

#[test]
fn cycle_witnesses_are_canonical_and_keep_independent_facts_separate() {
    let temp = TempDir::new("scc");
    let graph = synthetic_extracted(
        "synthetic",
        &["a", "b", "c", "d"],
        vec![
            synthetic_edge("synthetic::a", "synthetic::b::Item", "syntax"),
            synthetic_edge("synthetic::b", "synthetic::c::Item", "syntax"),
            synthetic_edge("synthetic::c", "synthetic::a::Item", "syntax"),
            synthetic_edge("synthetic::c", "synthetic::d::Item", "heuristic"),
        ],
    );
    let rules_path = write_rules(
        &temp,
        r#"
[components]
a = "synthetic::a"
b = "synthetic::b"
c = "synthetic::c"
d = "synthetic::d"

[allow]
a = []
b = ["c"]
c = ["a"]
d = []

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );
    let checked = rules::check(temp.path(), "synthetic", &rules_path, &graph).expect("rules");
    let cycle = checked
        .findings
        .iter()
        .find(|finding| finding["rule"] == "modules.cycle")
        .expect("cycle finding");
    assert_eq!(cycle["members"], json!(["a", "b", "c"]));
    assert_eq!(cycle["path"], json!(["a", "b", "c", "a"]));
    assert_eq!(cycle["extraction"], "syntax");
    assert!(cycle["undeclared_edges"].as_array().is_some_and(|edges| {
        edges
            .iter()
            .any(|edge| edge["from"] == "a" && edge["to"] == "b")
    }));
    assert!(checked.findings.iter().any(|finding| {
        finding["rule"] == "modules.undeclared_dependency"
            && finding["from"] == "c"
            && finding["to"] == "d"
            && finding["extraction"] == "heuristic"
    }));
}

#[test]
fn parallel_raw_cycle_edges_preserve_heuristic_evidence() {
    let temp = TempDir::new("parallel-cycle");
    let graph = synthetic_extracted(
        "synthetic",
        &["a", "b"],
        vec![
            synthetic_edge("synthetic::a", "synthetic::b::A", "syntax"),
            synthetic_edge("synthetic::a", "synthetic::b::Z", "heuristic"),
            synthetic_edge("synthetic::b", "synthetic::a::A", "syntax"),
        ],
    );
    let path = write_rules(
        &temp,
        r#"[components]
a = "synthetic::a"
b = "synthetic::b"
[allow]
a = []
b = ["a"]
[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );
    let checked = rules::check(temp.path(), "synthetic", &path, &graph).expect("rules");
    assert_eq!(checked.findings.len(), 1);
    let cycle = &checked.findings[0];
    assert_eq!(cycle["rule"], "modules.cycle");
    assert_eq!(cycle["path"], json!(["a", "b", "a"]));
    assert_eq!(cycle["extraction"], "heuristic");
    assert_eq!(
        cycle["undeclared_edges"],
        json!([{"from":"a", "to":"b", "extraction":"heuristic"}])
    );
}

#[test]
fn disabling_cycles_does_not_hide_undeclared_dependencies() {
    let temp = TempDir::new("cycles-disabled");
    let graph = synthetic_extracted(
        "synthetic",
        &["a", "b"],
        vec![synthetic_edge(
            "synthetic::a",
            "synthetic::b::Item",
            "syntax",
        )],
    );
    let rules_path = write_rules(
        &temp,
        r#"
[components]
a = "synthetic::a"
b = "synthetic::b"

[allow]
a = []
b = []

[deny]
cycles = false
child_to_parent_private = true
foreign_internal = true
"#,
    );
    let checked = rules::check(temp.path(), "synthetic", &rules_path, &graph).expect("rules");
    assert!(
        checked
            .findings
            .iter()
            .any(|finding| finding["rule"] == "modules.undeclared_dependency")
    );
    assert!(checked.limitations.iter().any(|limitation| {
        limitation["code"] == "disabled_rule"
            && limitation["detail"]
                .as_str()
                .is_some_and(|detail| detail.contains("modules.cycle"))
    }));
}

#[test]
fn nested_child_to_parent_private_edges_ignore_allow_lists() {
    let temp = TempDir::new("nested-d2");
    let graph = synthetic_extracted(
        "synthetic",
        &["parent", "parent::child", "sibling"],
        vec![synthetic_edge(
            "synthetic::parent::child",
            "synthetic::sibling::Item",
            "syntax",
        )],
    );
    let rules_path = write_rules(
        &temp,
        r#"
[components]
parent = "synthetic::parent"
child = "synthetic::parent::child"
sibling = "synthetic::sibling"

[allow]
parent = []
child = ["sibling"]
sibling = []

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );
    let checked = rules::check(temp.path(), "synthetic", &rules_path, &graph).expect("rules");
    assert!(checked.findings.iter().any(|finding| {
        finding["rule"] == "modules.child_to_parent_private"
            && finding["from"] == "child"
            && finding["to"] == "crate::sibling::Item"
    }));
    assert!(
        !checked
            .findings
            .iter()
            .any(|finding| finding["rule"] == "modules.undeclared_dependency")
    );
}

#[test]
fn unknown_components_and_missing_rules_are_configuration_errors() {
    let temp = TempDir::new("bad-rules");
    let graph = synthetic_extracted("synthetic", &["a"], Vec::new());
    let unknown = write_rules(
        &temp,
        r#"
[components]
a = "synthetic::missing"

[allow]
a = []

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );
    assert!(
        rules::check(temp.path(), "synthetic", &unknown, &graph)
            .expect_err("unknown component must fail")
            .contains("undiscovered")
    );
    assert!(
        rules::check(
            temp.path(),
            "synthetic",
            &temp.path().join("missing-rha-modules.toml"),
            &graph
        )
        .expect_err("missing rules must fail")
        .contains("failed to canonicalize")
    );
}
