use std::path::{Path, PathBuf};

use serde_json::Value;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../xtask/tests/corpus/verifier-shape")
}

#[test]
fn every_supplementary_shape_fixture_has_exactly_its_registered_decision() {
    let root = corpus_root();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("verifier-shape corpus")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('S'))
        })
        .collect();
    files.sort();
    assert_eq!(files.len(), 188, "the registration has 188 S fixtures");

    let mut malformed = 0;
    let mut well_formed = 0;
    let mut failures = Vec::new();
    for path in &files {
        let fixture: Value = serde_json::from_slice(&std::fs::read(path).expect("fixture bytes"))
            .expect("fixture JSON");
        let expected = fixture["expected"]
            .as_object()
            .expect("expected decision object");
        let actual = rha_verifier::evaluate(&fixture);
        let actual_object = actual.as_object().expect("verifier decision object");

        if expected.contains_key("malformed") {
            malformed += 1;
        } else {
            well_formed += 1;
            assert!(
                !actual_object.contains_key("malformed"),
                "{} unexpectedly refused a well-formed input",
                path.display()
            );
        }

        let expected_keys: std::collections::BTreeSet<&str> =
            expected.keys().map(String::as_str).collect();
        let actual_keys: std::collections::BTreeSet<&str> =
            actual_object.keys().map(String::as_str).collect();
        if expected_keys != actual_keys {
            failures.push(format!(
                "{}: expected keys {expected_keys:?}, got {actual_keys:?}",
                path.display()
            ));
            continue;
        }
        let wrong: Vec<&str> = expected
            .keys()
            .filter_map(|key| (actual[key] != expected[key]).then_some(key.as_str()))
            .collect();
        if !wrong.is_empty() {
            failures.push(format!("{}: values differ for {wrong:?}", path.display()));
        }
    }

    assert_eq!(malformed, 153, "registered malformed count");
    assert_eq!(well_formed, 35, "registered well-formed count");
    println!("verifier-shape observed malformed={malformed} well_formed={well_formed}");
    assert!(
        failures.is_empty(),
        "{} of {} verifier-shape fixtures differed:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}
