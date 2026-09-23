use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use xtask::corpus::module::{validate_headline_reference, validate_random_report};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn headline_report(root: &Path, id: &str) -> (PathBuf, Value) {
    let fixture = root.join("xtask/tests/corpus/module/headline").join(id);
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["architecture", "--manifest-path"])
        .arg(fixture.join("Cargo.toml"))
        .args(["--module-rules"])
        .arg(fixture.join("rha-modules.toml"))
        .args(["--format", "json"])
        .current_dir(root)
        .output()
        .expect("headline checker");
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "headline checker did not emit JSON: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (fixture, report)
}

fn valid_random_report(root: &Path) -> Value {
    let registration_path = root.join("xtask/tests/corpus/module/registration.toml");
    let registration_bytes = std::fs::read(&registration_path).expect("registration");
    let registration: toml::Value =
        toml::from_str(std::str::from_utf8(&registration_bytes).expect("registration UTF-8"))
            .expect("registration TOML");
    let counts = registration["counts"]
        .as_table()
        .expect("registered counts")
        .iter()
        .map(|(key, value)| (key.clone(), value.as_integer().expect("integer count")))
        .collect::<std::collections::BTreeMap<_, _>>();
    let ids = (0..256)
        .map(|index| format!("R{index:04}"))
        .collect::<Vec<_>>();
    let cases = ids
        .iter()
        .map(|id| json!({"id": id, "passed": true, "reasons": []}))
        .collect::<Vec<_>>();
    let registration_sha256 = xtask::util::sha256_hex(&registration_bytes);
    json!({
        "outcome": "passed",
        "registration": {
            "path": "xtask/tests/corpus/module/registration.toml",
            "sha256": registration_sha256,
            "counts": counts,
            "ids": ids,
        },
        "digests": {"registration_sha256": registration_sha256},
        "counts": {
            "random_cases": 256,
            "random_edges": 5229,
            "random_test_edges": 512,
            "random_heuristic_edges": 116,
            "random_limitations": 0,
            "optional_cycles": 64,
            "optional_top_level_findings": 192,
            "optional_top_level_undeclared_edges": 64,
            "optional_subsumed_undeclared_edges": 64,
            "optional_total_undeclared_edge_facts": 128,
            "random_rust_files": 1228,
            "random_source_map_files": 1740,
        },
        "cases": cases,
        "mismatch_ids": [],
        "reasons": [],
    })
}

#[test]
fn random_report_controls_reject_registered_count_drift() {
    let root = root();
    let mut report = valid_random_report(&root);
    validate_random_report(&report).expect("complete synthetic report is the positive control");
    report["counts"]["random_cases"] = json!(250);
    let error = validate_random_report(&report).expect_err("250 random cases must fail");
    assert!(
        error.to_string().contains("random_cases"),
        "count error should identify the drift: {error}"
    );
}

#[test]
fn all_registered_headline_extractions_match_the_supplementary_reference() {
    let root = root();
    let corpus = root.join("xtask/tests/corpus/module");
    let payloads = xtask::corpus::module::verify_inventory(&corpus).expect("frozen inventory");
    xtask::corpus::module::verify_registration_contract(&corpus, &payloads).expect("registration");
    let mut ids = std::fs::read_dir(corpus.join("headline"))
        .expect("headline directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("UTF-8 id")
        })
        .collect::<Vec<_>>();
    ids.sort();
    let mut edges = 0;
    let mut limitations = 0;
    for id in &ids {
        let (fixture, report) = headline_report(&root, id);
        validate_headline_reference(&fixture, &report)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        edges += report["module_edges"].as_array().expect("edges").len();
        limitations += report["limitations"]
            .as_array()
            .expect("limits")
            .iter()
            .filter(|limit| limit["code"] != "source_approximation")
            .count();
    }
    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(corpus.join("registration.toml")).expect("registration"),
    )
    .expect("TOML");
    assert_eq!(
        ids.len() as i64,
        registration["counts"]["headline_fixtures"]
            .as_integer()
            .unwrap()
    );
    assert_eq!(
        edges as i64,
        registration["counts"]["headline_edges"]
            .as_integer()
            .unwrap()
    );
    assert_eq!(
        limitations as i64,
        registration["counts"]["headline_limitations"]
            .as_integer()
            .unwrap()
    );
    eprintln!(
        "supplementary headline extraction: {} fixtures, {edges} edges, {limitations} limitations; exact reference match",
        ids.len()
    );
}

#[test]
fn headline_reference_controls_reject_missing_registered_facts() {
    let root = root();

    let (m07_fixture, m07_report) = headline_report(&root, "M07");
    validate_headline_reference(&m07_fixture, &m07_report)
        .expect("actual M07 headline report should match its reference");
    let mut missing_edge = m07_report.clone();
    missing_edge["module_edges"]
        .as_array_mut()
        .expect("M07 module edges")
        .pop();
    assert!(
        validate_headline_reference(&m07_fixture, &missing_edge).is_err(),
        "dropping a registered M07 edge must fail"
    );

    let (em_fixture, em_report) = headline_report(&root, "EM-M01");
    validate_headline_reference(&em_fixture, &em_report)
        .expect("actual EM-M01 headline report should match its reference");
    let mut missing_limitation = em_report.clone();
    missing_limitation["limitations"]
        .as_array_mut()
        .expect("EM-M01 limitations")
        .retain(|limitation| limitation["code"] != "macro_definition");
    assert!(
        validate_headline_reference(&em_fixture, &missing_limitation).is_err(),
        "dropping a registered EM-M01 limitation must fail"
    );
}
