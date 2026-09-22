//! The registered record corpus (P-B stage 1), graded exactly: the lint's
//! reason-code set must equal each fixture's, and accepted records have none.

use std::path::Path;

#[test]
fn every_registered_record_is_linted_as_registered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let l0 = xtask::record_lint::l0_ids(&root).expect("policy");
    let dir = root.join("xtask/tests/corpus/records");
    let mut cases: Vec<_> = std::fs::read_dir(&dir)
        .expect("corpus")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert_eq!(cases.len(), 80, "the registration has 80 records");
    let mut failures = Vec::new();
    for case in &cases {
        let e: serde_json::Value =
            serde_json::from_slice(&std::fs::read(case.join("EXPECTED.json")).expect("expected"))
                .expect("json");
        let got = xtask::record_lint::lint(
            &case.join(e["record"].as_str().expect("record")),
            e["kind"].as_str().expect("kind"),
            &l0,
        );
        let want: std::collections::BTreeSet<&str> = e["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        if got != want || want.is_empty() != (e["accept"] == true) {
            failures.push(format!(
                "{}: want {want:?} got {got:?}",
                e["id"].as_str().unwrap_or("?")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 80 records linted differently:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
