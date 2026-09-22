//! H4 boundary and negative controls. The public manifest remains fixed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use xtask::corpus::{Case, Manifest, fixture, grade, runner};

fn manifest() -> Manifest {
    Manifest::parse(include_str!("corpus/manifest.toml")).expect("public manifest")
}

fn case(id: &str) -> Case {
    manifest()
        .cases
        .into_iter()
        .find(|c| c.id == id)
        .expect("registered case")
}

fn report(findings: Vec<Value>) -> Value {
    let errors = findings.iter().filter(|f| f["severity"] == "error").count();
    let warnings = findings
        .iter()
        .filter(|f| f["severity"] == "warning")
        .count();
    json!({"schema_version": 1, "findings": findings, "harness_edges": [], "summary": {"errors": errors, "warnings": warnings, "outcome": if errors == 0 { "passed" } else { "failed" }}})
}

fn finding(case: &Case) -> Value {
    let mut value = json!(case.witness);
    value["severity"] = json!("error");
    if value["from"].is_null() {
        value["from"] = json!("subject");
    }
    value
}

#[test]
fn missing_wrong_and_new_witness_fields_fail_with_a_valid_control() {
    let mut case = case("C11");
    let good = finding(&case);
    assert!(grade::architecture(&case, Some(1), &report(vec![good.clone()])).passed);
    for key in case.witness.keys() {
        let mut missing = good.clone();
        missing.as_object_mut().expect("object").remove(key);
        assert!(
            !grade::architecture(&case, Some(1), &report(vec![missing])).detected,
            "{key}"
        );
        let mut wrong = good.clone();
        wrong[key] = json!("wrong");
        assert!(
            !grade::architecture(&case, Some(1), &report(vec![wrong])).detected,
            "{key}"
        );
    }
    case.witness.insert(
        "future_key".to_owned(),
        toml::Value::String("required".to_owned()),
    );
    assert!(!grade::architecture(&case, Some(1), &report(vec![good])).detected);
}

#[test]
fn detection_and_unmatched_warnings_are_counted_independently() {
    let case = case("C01");
    let warning = json!({"rule":"another.rule", "severity":"warning", "from":"core-a"});
    let grade = grade::architecture(
        &case,
        Some(1),
        &report(vec![finding(&case), warning.clone()]),
    );
    assert!(grade.detected);
    assert!(!grade.passed);
    assert_eq!(grade.false_alarms, vec![warning]);
}

#[test]
fn a_violation_downgraded_to_warning_is_not_enforced() {
    let case = case("C01");
    let mut value = finding(&case);
    value["severity"] = json!("warning");
    let graded = grade::architecture(&case, Some(0), &report(vec![value]));
    assert!(!graded.detected);
    assert!(!graded.passed);
}

#[test]
fn false_alarm_headline_includes_unmatched_findings_on_detect_cases() {
    let records = vec![
        json!({"id":"C13", "expected":"detect", "detector":"xtask architecture", "grade":{"detected":true,"passed":false,"false_alarms":[{"rule":"adapter.foreign_core"}]}}),
        json!({"id":"L01", "expected":"no_alarm", "detector":"xtask architecture", "grade":{"passed":true,"false_alarms":[]}}),
        json!({"id":"R01", "expected":"detect", "detector":"cargo check --offline", "grade":{"detected":true,"passed":true,"false_alarms":[]}}),
    ];
    let summary = runner::summarize(&records);
    assert_eq!(summary["false_alarm"]["alarms"], 1);
    assert_eq!(summary["false_alarm"]["legitimate_cases_with_alarms"], 0);
    assert_eq!(summary["false_alarm"]["legitimate"], 1);
    assert_eq!(summary["detection"]["violations"], 2);
    assert_eq!(summary["compiler"]["detected"], 1);
    assert_eq!(summary["outcome"], "failed");
}

#[test]
fn tool_errors_bad_json_and_exit_disagreement_never_score() {
    for id in ["C01", "L01", "EM-C01"] {
        let case = case(id);
        for value in [
            Value::Null,
            json!({"findings":[], "summary":{"error_class":"tool_error"}}),
        ] {
            assert!(!grade::architecture(&case, Some(3), &value).passed);
        }
        assert!(!grade::architecture(&case, Some(3), &report(vec![])).passed);
        assert!(!grade::architecture(&case, None, &report(vec![])).passed);
        assert!(!grade::architecture(&case, Some(1), &report(vec![])).passed);
    }
    assert!(grade::architecture(&case("L01"), Some(0), &report(vec![])).passed);
}

#[test]
fn l07_requires_listed_fact_and_rejects_it_as_an_alarm() {
    let case = case("L07");
    assert!(!grade::architecture(&case, Some(0), &report(vec![])).passed);
    let fact = json!({"rule":"dir.non_root_to_adapter", "from":"core-a", "to":"adapter-x", "kind":"dev", "severity":"note"});
    let mut good = report(vec![]);
    good["harness_edges"] = json!([fact.clone()]);
    assert!(grade::architecture(&case, Some(0), &good).passed);
    let mut error_fact = good.clone();
    error_fact["harness_edges"][0]["severity"] = json!("error");
    assert!(!grade::architecture(&case, Some(0), &error_fact).passed);
    let mut malformed = good.clone();
    malformed["harness_edges"] = json!(["bad"]);
    assert!(!grade::architecture(&case, Some(0), &malformed).passed);
    good["harness_edges"][0]["to"] = json!("wrong");
    assert!(!grade::architecture(&case, Some(0), &good).passed);
    let bad = grade::architecture(&case, Some(0), &report(vec![fact]));
    assert!(!bad.passed);
    assert_eq!(bad.false_alarms.len(), 1);
}

#[test]
fn expected_miss_surprises_are_terminal_and_keep_the_hole() {
    let case = case("EM-C01");
    assert!(grade::architecture(&case, Some(0), &report(vec![])).documented_miss);
    let graded = grade::architecture(&case, Some(1), &report(vec![finding(&case)]));
    assert!(graded.unexpected_detection);
    assert!(!graded.passed);
    assert!(case.hole.is_some());
    let other = json!({"rule":"unrelated", "severity":"error", "from":"core-a"});
    let graded = grade::architecture(&case, Some(1), &report(vec![other]));
    assert!(!graded.unexpected_detection);
    assert!(!graded.passed);
    assert_eq!(graded.false_alarms.len(), 1);
}

#[test]
fn compiler_needs_code_crate_full_name_and_failure_exit() {
    let case = case("R01");
    let good = json!(case.witness);
    assert!(grade::compiler(&case, Some(101), std::slice::from_ref(&good)).passed);
    assert!(!grade::compiler(&case, Some(0), std::slice::from_ref(&good)).passed);
    assert!(!grade::compiler(&case, Some(101), &[]).passed);
    for key in case.witness.keys() {
        let mut wrong = good.clone();
        wrong[key] = json!("wrong");
        assert!(!grade::compiler(&case, Some(101), &[wrong]).passed);
    }
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rha-h4-{}-{}-{}",
            std::process::id(),
            xtask::util::UtcTime::now().compact(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("fresh temporary directory");
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn template() -> &'static [u8] {
    include_bytes!("../templates/core-clippy.toml")
}

fn architecture(workspace: &Path) -> (Option<i32>, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["architecture", "--manifest-path"])
        .arg(workspace.join("Cargo.toml"))
        .arg("--rules")
        .arg(workspace.join("rha-crates.toml"))
        .args(["--format", "json"])
        .output()
        .expect("architecture CLI");
    (
        output.status.code(),
        serde_json::from_slice(&output.stdout).expect("JSON report"),
    )
}

#[test]
fn real_metadata_preserves_all_dependency_shapes_and_outside_identity() {
    let manifest = manifest();
    let temp = Temp::new();
    for id in ["C15", "C16", "C17", "C18", "C19", "EM-C01", "L07"] {
        let case = manifest.cases.iter().find(|c| c.id == id).expect("case");
        let workspace =
            fixture::generate(case, &manifest, &temp.0.join(id), template()).expect("generate");
        let (exit, report) = architecture(&workspace);
        let graded = grade::architecture(case, exit, &report);
        assert!(graded.passed, "{id}: {graded:?}\n{report}");
        assert_eq!(
            report["subject"]["manifest_path"],
            json!(workspace.join("Cargo.toml"))
        );
        assert!(report["tool"]["git_rev"].is_string());
        assert_eq!(
            report["tool"]["git_dirty"],
            json!(
                xtask::util::git_identity(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .expect("root")
                )
                .map(|i| i.dirty)
            )
        );
        if id == "C15" {
            assert_eq!(report["findings"][0]["witness"]["target"], "cfg(unix)");
        }
        if id == "C16" {
            assert_eq!(report["findings"][0]["witness"]["optional"], true);
        }
        if id == "C17" {
            assert_eq!(report["findings"][0]["witness"]["rename"], "ax");
        }
        if id == "EM-C01" {
            assert_eq!(
                report["classification"]
                    .as_array()
                    .expect("classification")
                    .len(),
                1
            );
        }
    }
}

#[test]
fn rule_specific_witness_fields_survive_the_real_cli() {
    let manifest = manifest();
    let temp = Temp::new();
    for id in [
        "C04", "C05", "C08", "C09", "C10", "C11", "C13", "C14", "C20",
    ] {
        let case = manifest.cases.iter().find(|c| c.id == id).expect("case");
        let workspace =
            fixture::generate(case, &manifest, &temp.0.join(id), template()).expect("generate");
        let (exit, report) = architecture(&workspace);
        let graded = grade::architecture(case, exit, &report);
        assert!(graded.detected, "{id}: {graded:?}\n{report}");
        // Until CHG-004.6 this asserted that C13's accurate extra warning
        // failed grading. The DP-1.1c amendment registered it (§9.14): it is
        // now a registered fact, and nothing else about the case changed.
        assert!(graded.passed, "{id}: {graded:?}");
        if id == "C13" {
            assert_eq!(graded.registered_facts.len(), 1);
            assert_eq!(graded.registered_facts[0]["rule"], "adapter.foreign_core");
            assert!(graded.false_alarms.is_empty());
        } else {
            assert!(graded.registered_facts.is_empty(), "{id}");
        }
    }
}

#[test]
fn failed_cases_downgrade_every_named_cell_without_changing_manifest() {
    let manifest = manifest();
    let records =
        vec![json!({"id":"C01", "cells":["law3-d1", "law6-d5"], "grade":{"passed":false}})];
    let map = runner::enforcement_map(&manifest, &records, "evidence/test.json", None);
    for id in ["law3-d1", "law6-d5"] {
        assert!(
            map.lines()
                .any(|line| line.contains(id) && line.contains("downgraded"))
        );
    }
    assert!(
        map.lines()
            .any(|line| line.contains("`law5`") && line.contains("not_run"))
    );
    let map = runner::enforcement_map(
        &manifest,
        &[json!({"id":"C10","cells":["classification"],"grade":{"passed":false}})],
        "evidence/test.json",
        None,
    );
    assert_eq!(
        map.lines()
            .filter(|line| line.contains("downgraded"))
            .count(),
        manifest.cells.len()
    );
}

#[test]
fn a_record_of_an_earlier_manifest_claims_no_cell() {
    let manifest = manifest();
    let passing = vec![json!({"id":"C01", "cells":["law3-d1"], "grade":{"passed":true}})];
    let map = runner::enforcement_map(&manifest, &passing, "evidence/old.json", Some("abc"));
    assert!(map.contains("**Stale.**"));
    assert!(!map.contains("validated"), "{map}");
    let fresh = runner::enforcement_map(&manifest, &passing, "evidence/new.json", None);
    assert!(fresh.contains("validated"));
}

#[test]
fn deterministic_generator_preserves_declared_source_and_rejects_escape() {
    let manifest = manifest();
    let temp = Temp::new();
    let case = case("EM-C02");
    let first = temp.0.join("first");
    let second = temp.0.join("second");
    let workspace = fixture::generate(&case, &manifest, &first, template()).expect("generate");
    fixture::generate(&case, &manifest, &second, template()).expect("generate again");
    assert_eq!(
        fixture::digests(&first).expect("digests"),
        fixture::digests(&second).expect("digests")
    );
    let source = std::fs::read_to_string(workspace.join("core-a/src/lib.rs")).expect("source");
    assert!(source.contains(case.crates[0].body.as_deref().expect("body")));
    assert!(source.contains("#![forbid("));
    let mut escape = manifest
        .cases
        .iter()
        .find(|c| c.id == "C19")
        .expect("C19")
        .clone();
    escape.outside_crates[0].at = "../../escape".to_owned();
    assert!(fixture::generate(&escape, &manifest, &temp.0.join("unsafe"), template()).is_err());
    assert!(!temp.0.join("escape").exists());
}

#[test]
fn report_gate_rejects_missing_or_wrong_producer_subject_and_coverage() {
    let manifest = manifest();
    let temp = Temp::new();
    let case = case("C01");
    let workspace =
        fixture::generate(&case, &manifest, &temp.0.join("C01"), template()).expect("fixture");
    let (_, good) = architecture(&workspace);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tool root");
    let tool = xtask::graph::report::tool_identity(root);
    assert!(runner::validate_report(&case, &workspace, &tool, &good).is_ok());
    for pointer in [
        "/tool/git_rev",
        "/tool/git_dirty",
        "/subject/workspace_root",
        "/subject/manifest_path",
        "/subject/metadata_mode",
        "/subject/rules_digest",
        "/classification/0/name",
        "/classification/0/manifest_path",
        "/edges_examined/normal",
        "/findings/0/manifest_path",
        "/findings/0/witness/edge",
        "/findings/0/witness/declared_in",
    ] {
        for replacement in [Value::Null, json!("wrong")] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).expect("field exists") = replacement;
            assert!(
                runner::validate_report(&case, &workspace, &tool, &bad).is_err(),
                "{pointer}"
            );
        }
    }
    for field in [
        "classification",
        "edges_examined",
        "findings",
        "harness_edges",
    ] {
        let mut bad = good.clone();
        bad.as_object_mut().expect("object").remove(field);
        assert!(
            runner::validate_report(&case, &workspace, &tool, &bad).is_err(),
            "{field}"
        );
    }
}

#[test]
fn edge_kind_and_cycle_path_cannot_be_omitted_or_faked() {
    let manifest = manifest();
    let temp = Temp::new();
    for id in ["C01", "C05"] {
        let case = case(id);
        let workspace =
            fixture::generate(&case, &manifest, &temp.0.join(id), template()).expect("fixture");
        let (_, good) = architecture(&workspace);
        let tool = good["tool"].clone();
        assert!(runner::validate_report(&case, &workspace, &tool, &good).is_ok());
        if id == "C01" {
            for kind in [Value::Null, json!(true), json!({}), json!("unknown")] {
                let mut bad = good.clone();
                bad["findings"][0]["kind"] = kind;
                assert!(runner::validate_report(&case, &workspace, &tool, &bad).is_err());
            }
        } else {
            for edge in [
                "not-a-cycle",
                "cycle: core-a -> core-b",
                "cycle: core-a -> ghost -> core-a",
            ] {
                let mut bad = good.clone();
                bad["findings"][0]["witness"]["edge"] = json!(edge);
                assert!(runner::validate_report(&case, &workspace, &tool, &bad).is_err());
            }
        }
    }
}

#[test]
fn materialize_prunes_inputs_the_registration_no_longer_produces() {
    // Before CHG-004.5 `corpus generate` only wrote expected files, so a
    // committed input the manifest stopped producing stayed behind and the
    // drift check could never be cleared by the command itself.
    let temp = Temp::new();
    let base = temp.0.join("committed");
    let expected: BTreeMap<PathBuf, Vec<u8>> = [
        (
            PathBuf::from("violations/X/Cargo.toml"),
            b"[workspace]\n".to_vec(),
        ),
        (
            PathBuf::from("violations/X/core-a/src/lib.rs"),
            b"pub fn v() {}\n".to_vec(),
        ),
    ]
    .into_iter()
    .collect();
    std::fs::create_dir_all(base.join("violations/X/core-a/src")).expect("dirs");
    std::fs::write(base.join("violations/X/core-a/build.rs"), "fn main() {}\n").expect("stray");
    std::fs::create_dir_all(base.join("violations/OLD/src")).expect("old case");
    std::fs::write(base.join("violations/OLD/src/lib.rs"), "").expect("old input");
    std::fs::create_dir_all(base.join("violations/X/target")).expect("cargo output dir");
    std::fs::write(base.join("violations/X/target/keep"), "").expect("cargo output");
    std::fs::write(base.join("violations/X/Cargo.lock"), "").expect("lockfile");

    let removed = fixture::materialize(&base, &expected).expect("materialize");
    assert_eq!(
        removed,
        vec![
            PathBuf::from("violations/OLD/src/lib.rs"),
            PathBuf::from("violations/X/core-a/build.rs"),
        ]
    );
    assert!(
        !base.join("violations/OLD").exists(),
        "a case directory emptied by pruning goes with its last input"
    );
    assert!(
        base.join("violations/X/target/keep").exists()
            && base.join("violations/X/Cargo.lock").exists(),
        "Cargo output is neither an input nor pruned"
    );
    assert!(
        fixture::drift(&base, &expected)
            .expect("compare")
            .is_empty()
    );
    assert!(
        fixture::materialize(&base, &expected)
            .expect("second run")
            .is_empty(),
        "a converged tree prunes nothing"
    );
}

#[test]
fn execution_errors_keep_their_class() {
    // An unsupported detector is the registration's fault; a detector that
    // cannot be spawned is the environment's. Before CHG-004.5 both were one
    // untyped string in the record.
    let temp = Temp::new();
    let mut unsupported = case("R01");
    unsupported.detector = Some("python".to_owned());
    let Err(error) = runner::execute(&temp.0, &unsupported, &temp.0, &json!({})) else {
        panic!("an unsupported detector produces no observation");
    };
    assert_eq!(error.class, "config_error");
    assert!(error.message.contains("python"), "{}", error.message);

    let missing = temp.0.join("does-not-exist");
    let Err(error) = runner::execute(&temp.0, &case("R01"), &missing, &json!({})) else {
        panic!("a detector that cannot start produces no observation");
    };
    assert_eq!(error.class, "tool_error");
}

#[test]
fn a_registered_fact_is_neither_a_detection_nor_a_false_alarm() {
    // C13 registers adapter.foreign_core on adapter-x -> core-b (CHG-004.6).
    let c13 = case("C13");
    let fact = json!({"rule":"adapter.foreign_core", "severity":"warning", "from":"adapter-x", "crate":"adapter-x", "to":"core-b"});
    let graded = grade::architecture(&c13, Some(1), &report(vec![finding(&c13), fact.clone()]));
    assert!(graded.detected && graded.passed, "{:?}", graded.reasons);
    assert_eq!(graded.registered_facts, vec![fact.clone()]);
    assert!(graded.false_alarms.is_empty());

    // Negative controls: a changed key is not the registered fact, and a
    // missing key is not either. A finding without a rule is malformed and
    // rejected before grading, so the missing-key control covers the others.
    for key in ["rule", "crate", "to"] {
        let mut wrong = fact.clone();
        wrong[key] = json!("other");
        let graded = grade::architecture(&c13, Some(1), &report(vec![finding(&c13), wrong]));
        assert!(
            !graded.passed && graded.false_alarms.len() == 1,
            "{key} changed"
        );
    }
    for key in ["crate", "to"] {
        let mut missing = fact.clone();
        missing.as_object_mut().expect("object").remove(key);
        let graded = grade::architecture(&c13, Some(1), &report(vec![finding(&c13), missing]));
        assert!(
            !graded.passed && graded.false_alarms.len() == 1,
            "{key} missing"
        );
    }

    // A registered fact cannot stand in for the required witness.
    let graded = grade::architecture(&c13, Some(0), &report(vec![fact.clone()]));
    assert!(!graded.detected && !graded.passed);
    assert_eq!(graded.registered_facts.len(), 1);

    // An unregistered extra finding still fails the case.
    let extra = json!({"rule":"another.rule", "severity":"warning", "from":"adapter-x"});
    let graded = grade::architecture(
        &c13,
        Some(1),
        &report(vec![finding(&c13), fact.clone(), extra.clone()]),
    );
    assert!(!graded.passed);
    assert_eq!(graded.false_alarms, vec![extra]);

    // Only C13 registers the fact; the same warning on C01 is a false alarm.
    let c01 = case("C01");
    let graded = grade::architecture(&c01, Some(1), &report(vec![finding(&c01), fact]));
    assert!(!graded.passed);
    assert_eq!(graded.false_alarms.len(), 1);
    assert!(graded.registered_facts.is_empty());
}
