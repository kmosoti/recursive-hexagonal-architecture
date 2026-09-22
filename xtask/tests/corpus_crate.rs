//! The complete committed crate corpus, exercised through the public command.
//!
//! Until CHG-004.6 this file pinned C13 as the one failed case. The DP-1.1c
//! amendment Kennedy approved registered C13's accurate `adapter.foreign_core`
//! warning as a fact, so the pin moved to the amended expectation: 22/22
//! detected, no unmatched finding, the warning under `registered_facts` and
//! nowhere else (§9.14; the previous assertion is in the change record).

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
fn full_committed_corpus_pins_observations_under_the_amended_registration() {
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
        Some(0),
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
    assert_eq!(record["summary"]["false_alarm"]["alarms"], 0);
    assert_eq!(record["summary"]["registered_facts"], 1);
    assert_eq!(record["summary"]["false_alarm"]["legitimate"], 11);
    assert_eq!(record["summary"]["expected_miss"]["documented"], 2);
    assert_eq!(record["summary"]["failed_cases"], json!([]));
    assert_eq!(record["summary"]["outcome"], "passed");
    assert_eq!(record["fixtures"]["drift"], json!([]));
    // The amendment chain: pre-registration, then EM-M03 (CHG-004), then C13
    // (CHG-004.6). Each link is owned by its approval decision and verified
    // from the manifest's bytes by reverting the amendment textually.
    let decision = |task_path: &str, id: &str| -> toml::Value {
        let task: toml::Value =
            toml::from_str(&std::fs::read_to_string(root.join(task_path)).expect("task record"))
                .expect("task TOML");
        task["decisions"]
            .as_array()
            .expect("decisions")
            .iter()
            .find(|row| row["id"].as_str() == Some(id))
            .cloned()
            .expect("approval decision")
    };
    let em_m03 = decision(
        ".rha/tasks/CHG-004-h4-crate-harness.toml",
        "approved-em-m03-cells",
    );
    let c13 = decision(
        ".rha/tasks/CHG-004.6-c13-registration.toml",
        "approved-c13-registration",
    );
    let amendments = record["manifest"]["amendments"]
        .as_array()
        .expect("amendments");
    assert_eq!(amendments.len(), 3);
    assert_eq!(amendments[0]["change"], "CHG-002.2");
    assert_eq!(
        amendments[0]["parent_manifest_sha256"],
        xtask::corpus::runner::PRE_REGISTRATION_MANIFEST_SHA256
    );
    assert_eq!(amendments[1]["decision"], "approved-em-m03-cells");
    assert_eq!(amendments[2]["decision"], "approved-c13-registration");
    assert_eq!(
        amendments[1]["parent_manifest_sha256"],
        amendments[0]["corrected_manifest_sha256"]
    );
    for link in amendments {
        assert_ne!(link["commit_verified"], false, "{link}");
    }
    assert_ne!(amendments[0]["anchor_verified"], false);
    assert_eq!(
        record["manifest"]["correction_commit"],
        c13["commit"].as_str().expect("c13 commit")
    );
    let current = include_str!("corpus/manifest.toml");
    let fact_block = "# Registered fact (CHG-004.6, DP-1.1c amendment, approved by Kennedy 2026-09-20\n# after the data): the adapter's dependency on core-b, whose port it does not\n# implement, necessarily raises adapter.foreign_core, listed as a warning in\n# plan §5 before any data existed. Matched by the every-key rule; neither the\n# detection nor a false alarm.\nexpected_findings = [{ rule = \"adapter.foreign_core\", crate = \"adapter-x\", to = \"core-b\" }]\n";
    let sentence = ". A finding matching an entry of a case's expected_findings is a registered fact, neither a detection nor a false alarm.";
    assert_eq!(current.matches(fact_block).count(), 1);
    assert_eq!(current.matches(sentence).count(), 1);
    let before_c13 = current.replace(fact_block, "").replace(sentence, "");
    assert_eq!(
        xtask::util::sha256_hex(current.as_bytes()),
        c13["corrected_manifest_sha256"]
            .as_str()
            .expect("c13 corrected digest")
    );
    assert_eq!(
        xtask::util::sha256_hex(before_c13.as_bytes()),
        c13["parent_manifest_sha256"]
            .as_str()
            .expect("c13 parent digest")
    );
    assert_eq!(
        c13["parent_manifest_sha256"], em_m03["corrected_manifest_sha256"],
        "the C13 amendment's parent is the EM-M03 correction"
    );
    let corrected = "cells = [\"law3-d1\", \"law6-b3\"]\nseeded = \"a trait method";
    let old = "cells = [\"law6-b3\"]\nseeded = \"a trait method";
    assert_eq!(before_c13.matches(corrected).count(), 1);
    let prior = before_c13.replace(corrected, old);
    assert_eq!(
        xtask::util::sha256_hex(prior.as_bytes()),
        em_m03["parent_manifest_sha256"]
            .as_str()
            .expect("em-m03 parent digest")
    );
    let original: toml::Value = toml::from_str(include_str!("../../.rha/acceptances/CHG-002.toml"))
        .expect("CHG-002 acceptance");
    assert_eq!(
        record["pre_registration"]["revision"],
        original["subject"]["revision"]
            .as_str()
            .expect("registration revision")
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
            case["grade"]["passed"], true,
            "{}: {}",
            expected.id, case["grade"]
        );
        let registered = case["grade"]["registered_facts"]
            .as_array()
            .map_or(0, Vec::len);
        assert_eq!(
            registered,
            usize::from(expected.id == "C13"),
            "{}",
            expected.id
        );
        if expected.id == "C13" {
            let fact = &case["grade"]["registered_facts"][0];
            assert_eq!(fact["rule"], "adapter.foreign_core");
            assert_eq!(fact["crate"], "adapter-x");
            assert_eq!(fact["to"], "core-b");
            assert_eq!(fact["severity"], "warning");
        }
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
