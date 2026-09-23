//! The registered conformance corpus (P-B stage 1), graded exactly: every key
//! of each fixture's `expected` must equal the verifier's decision.

use std::path::Path;

#[test]
fn every_registered_fixture_is_decided_as_registered() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("corpus")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 152, "the registration has 152 fixtures");
    let mut failures = Vec::new();
    for path in &files {
        let fixture: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("read")).expect("json");
        let got = rha_verifier::evaluate(&fixture);
        let want = &fixture["expected"];
        let wrong: Vec<&String> = want
            .as_object()
            .expect("expected object")
            .keys()
            .filter(|k| got[k.as_str()] != want[k.as_str()])
            .collect();
        if !wrong.is_empty() {
            failures.push(format!(
                "{}: {wrong:?}",
                fixture["id"].as_str().unwrap_or("?")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} fixtures decided differently:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}
