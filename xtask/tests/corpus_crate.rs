//! The complete committed crate corpus, exercised through the public command.
//! C13 remains an explicit failed H4 result; this test does not waive it.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use xtask::corpus::{Manifest, fixture};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn registration() -> Manifest {
    Manifest::parse(include_str!("corpus/manifest.toml")).expect("registration")
}

#[test]
fn committed_fixtures_equal_generation_and_the_drift_check_detects_changes() {
    let root = root();
    let expected = fixture::expected_tree(&root, &registration()).expect("generation");
    assert!(
        fixture::drift(&root.join(fixture::COMMITTED_ROOT), &expected)
            .expect("compare")
            .is_empty()
    );
    let scratch = root
        .join("target/rha")
        .join(format!("drift-negative-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch");
    for (path, bytes) in &expected {
        let path = scratch.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent directories");
        std::fs::write(path, bytes).expect("copy generated input");
    }
    assert!(
        fixture::drift(&scratch, &expected)
            .expect("valid control")
            .is_empty()
    );
    let changed = PathBuf::from("violations/C01/core-a/src/lib.rs");
    std::fs::write(scratch.join(&changed), "pub fn wrong() {}\n").expect("inject drift");
    assert_eq!(
        fixture::drift(&scratch, &expected).expect("detect changed file"),
        vec![changed.clone()]
    );
    std::fs::write(scratch.join(&changed), &expected[&changed]).expect("restore valid input");
    let extra = PathBuf::from("unexpected.rs");
    std::fs::write(scratch.join(&extra), "").expect("inject extra file");
    assert_eq!(
        fixture::drift(&scratch, &expected).expect("detect extra file"),
        vec![extra.clone()]
    );
    std::fs::remove_file(scratch.join(extra)).expect("remove injected file");
    std::fs::remove_file(scratch.join(&changed)).expect("inject missing file");
    assert_eq!(
        fixture::drift(&scratch, &expected).expect("detect missing file"),
        vec![changed]
    );
    std::fs::remove_dir_all(scratch).expect("remove scratch");
}

#[test]
fn full_committed_corpus_pins_observations_and_preserves_the_h4_failure() {
    let root = root();
    let directory = root.join("target/rha").join(format!(
        "corpus-integration-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let map_before = std::fs::read(root.join("docs/enforcement-map.md")).ok();
    let output = xtask::util::command(env!("CARGO_BIN_EXE_xtask"))
        .args(["corpus", "run", "--level", "crate", "--evidence"])
        .arg(&directory)
        .current_dir(&root)
        .output()
        .expect("run complete public corpus");
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let files: Vec<_> = std::fs::read_dir(&directory)
        .expect("evidence directory")
        .map(|entry| entry.expect("entry").path())
        .collect();
    assert_eq!(files.len(), 1);
    let record: Value = serde_json::from_slice(&std::fs::read(&files[0]).expect("record bytes"))
        .expect("record JSON");
    assert_eq!(
        record["summary"]["detection"],
        json!({"detected":22,"violations":22})
    );
    assert_eq!(
        record["summary"]["checker"],
        json!({"detected":21,"cases":21})
    );
    assert_eq!(
        record["summary"]["compiler"],
        json!({"detected":1,"cases":1})
    );
    assert_eq!(record["summary"]["false_alarm"]["alarms"], 1);
    assert_eq!(record["summary"]["false_alarm"]["legitimate"], 11);
    assert_eq!(record["summary"]["expected_miss"]["documented"], 2);
    assert_eq!(record["summary"]["failed_cases"], json!(["C13"]));
    assert_eq!(record["summary"]["outcome"], "failed");
    assert_eq!(record["fixtures"]["drift"], json!([]));
    let correction = record["manifest"]["correction_commit"]
        .as_str()
        .expect("correction commit");
    let manifest_at = |revision: &str| {
        let source = xtask::util::command_stdout(
            &root,
            &[
                "git",
                "show",
                &format!("{revision}:{}", xtask::corpus::MANIFEST_PATH),
            ],
        )
        .expect("historical manifest");
        toml::from_str::<toml::Value>(&source).expect("historical TOML")
    };
    let mut before = manifest_at(&format!("{correction}^"));
    let after = manifest_at(correction);
    let row = before["case"]
        .as_array_mut()
        .expect("cases")
        .iter_mut()
        .find(|row| row["id"].as_str() == Some("EM-M03"))
        .expect("EM-M03");
    assert_eq!(
        row["cells"],
        toml::Value::Array(vec![toml::Value::String("law6-b3".to_owned())])
    );
    row["cells"] = toml::Value::Array(vec![
        toml::Value::String("law3-d1".to_owned()),
        toml::Value::String("law6-b3".to_owned()),
    ]);
    assert_eq!(
        before, after,
        "cited correction changed exactly the approved row"
    );
    let cases = record["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 35);
    for expected in registration()
        .cases
        .iter()
        .filter(|case| case.level == xtask::corpus::Level::Crate)
    {
        let matching: Vec<_> = cases
            .iter()
            .filter(|case| case["id"] == expected.id)
            .collect();
        assert_eq!(matching.len(), 1, "{}", expected.id);
        let case = matching[0];
        assert_eq!(
            case["grade"]["passed"],
            expected.id != "C13",
            "{}: {}",
            expected.id,
            case["grade"]
        );
        assert_eq!(
            case["fixture_path"],
            json!(Path::new(fixture::COMMITTED_ROOT).join(fixture::case_path(expected)))
        );
        if expected.detector.is_none() {
            assert_eq!(
                &case["argv"].as_array().expect("command")[..3],
                &[json!("cargo"), json!("xtask"), json!("architecture")]
            );
            assert_eq!(case["report"]["tool"], record["producer"]["checker"]);
        } else {
            assert_eq!(case["exit_status"], 101);
            assert_eq!(
                case["report"]["compiler_observations"][0]["error_code"],
                "E0603"
            );
        }
        if expected.expected == xtask::corpus::Expected::ExpectedMiss {
            assert_eq!(case["hole"], json!(expected.hole));
        }
    }
    assert_eq!(
        std::fs::read(root.join("docs/enforcement-map.md")).ok(),
        map_before,
        "only corpus docs generation owns the map"
    );
}
