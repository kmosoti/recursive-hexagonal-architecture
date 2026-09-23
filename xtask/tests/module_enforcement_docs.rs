use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xtask::corpus::{MANIFEST_PATH, Manifest, runner};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn manifest() -> Manifest {
    let root = root();
    Manifest::parse(&std::fs::read_to_string(root.join(MANIFEST_PATH)).expect("frozen manifest"))
        .expect("frozen manifest parses")
}

fn evidence(id: &str, cells: &[&str], passed: bool) -> Value {
    json!({
        "id": id,
        "cells": cells,
        "grade": {"passed": passed}
    })
}

fn project(crate_cases: &[Value], module_cases: &[Value]) -> String {
    runner::levels(
        &manifest(),
        runner::LevelEvidence {
            cases: crate_cases,
            evidence: "evidence/h4-crate/current.json",
            stale: None,
        },
        runner::LevelEvidence {
            cases: module_cases,
            evidence: "evidence/h4-module/current.json",
            stale: None,
        },
    )
}

fn row<'a>(map: &'a str, cell: &str) -> &'a str {
    map.lines()
        .find(|line| line.starts_with(&format!("| `{cell}` |")))
        .expect("enforcement-map row")
}

#[test]
fn missing_module_does_not_erase_a_valid_crate_projection() {
    let map = project(&[evidence("C01", &["law3-d1"], true)], &[]);
    let law3 = row(&map, "law3-d1");
    assert!(law3.contains("validated on registered crate cases only"));
    assert!(law3.contains("not_run — no registered module cases"));
}

#[test]
fn stale_crate_and_module_records_are_independent() {
    let crate_stale = runner::levels(
        &manifest(),
        runner::LevelEvidence {
            cases: &[evidence("C01", &["law3-d1"], true)],
            evidence: "evidence/h4-crate/old.json",
            stale: Some("old-crate"),
        },
        runner::LevelEvidence {
            cases: &[evidence("M02", &["law3-d1"], true)],
            evidence: "evidence/h4-module/current.json",
            stale: None,
        },
    );
    let law3 = row(&crate_stale, "law3-d1");
    assert!(law3.contains("latest crate H4 record graded manifest sha256:old-crate"));
    assert!(law3.contains("corpus validated (advisory; not accepted)"));

    let module_stale = runner::levels(
        &manifest(),
        runner::LevelEvidence {
            cases: &[evidence("C01", &["law3-d1"], true)],
            evidence: "evidence/h4-crate/current.json",
            stale: None,
        },
        runner::LevelEvidence {
            cases: &[evidence("M02", &["law3-d1"], true)],
            evidence: "evidence/h4-module/old.json",
            stale: Some("old-module"),
        },
    );
    let law3 = row(&module_stale, "law3-d1");
    assert!(law3.contains("validated on registered crate cases only"));
    assert!(law3.contains("latest module H4 record graded manifest sha256:old-module"));
}

#[test]
fn crate_classification_failure_does_not_downgrade_module() {
    let map = project(
        &[evidence("C10", &["classification"], false)],
        &[evidence("M02", &["law3-d1"], true)],
    );
    let law3 = row(&map, "law3-d1");
    assert!(law3.contains("review — downgraded"));
    assert!(law3.contains("corpus validated (advisory; not accepted)"));
}

#[test]
fn module_failure_does_not_downgrade_crate() {
    let map = project(
        &[evidence("C01", &["law3-d1"], true)],
        &[evidence("M02", &["law3-d1"], false)],
    );
    let law3 = row(&map, "law3-d1");
    assert!(law3.contains("validated on registered crate cases only"));
    assert!(law3.contains("review — downgraded"));
}

#[test]
fn expected_miss_hole_is_escaped_and_displayed() {
    let mut expected_miss = evidence("EM-C01", &["law5"], true);
    expected_miss["hole"] = json!("§4.1 | known\nhole");
    let map = project(&[expected_miss], &[]);
    assert!(map.contains("§4.1 \\| known hole"));
}

#[test]
fn supplemental_random_failure_downgrades_only_its_cells() {
    let map = project(
        &[],
        &[
            evidence("M01", &["law6-b3"], true),
            json!({
                "id": "random-module-reference",
                "cells": ["law6-b3", "d2", "law3-d1", "law2-d3"],
                "grade": {"passed": false},
                "report": {"counts": {"cases": 256}}
            }),
        ],
    );
    assert!(row(&map, "law6-b3").contains("review — downgraded"));
    assert!(row(&map, "law6-b3").contains("256 supplementary cases"));
    assert!(row(&map, "law5").contains("not_run — no registered module cases"));
}

#[test]
fn unrelated_failure_does_not_contaminate_other_cells_and_reference_is_not_validation() {
    let map = project(
        &[],
        &[
            evidence("M01", &["law6-b3"], true),
            evidence("unrelated", &["law5"], false),
            evidence("X-M01", &["law6-b3"], false),
        ],
    );
    assert!(row(&map, "law6-b3").contains("corpus validated (advisory; not accepted)"));
    assert!(row(&map, "law6-b3").contains("X-M01 (reference)"));
    assert!(row(&map, "law5").contains("review — downgraded"));
}

#[test]
fn projection_is_deterministic_for_synthetic_evidence() {
    let crate_cases = [evidence("C01", &["law3-d1"], true)];
    let module_cases = [evidence("M02", &["law3-d1"], true)];
    assert_eq!(
        project(&crate_cases, &module_cases),
        project(&crate_cases, &module_cases)
    );
}
