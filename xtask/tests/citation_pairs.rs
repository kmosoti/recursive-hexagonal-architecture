use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

const ZERO: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rha-{label}-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&path).expect("temp directory");
    path
}

fn run_lint(record: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["rha", "lint"])
        .arg(record)
        .current_dir(root())
        .output()
        .expect("spawn xtask")
}

#[test]
fn task_citations_are_unchecked_without_a_subject_but_not_bare_accepted() {
    let source_path = root().join(".rha/tasks/CHG-007-module-level.toml");
    let source = std::fs::read_to_string(&source_path).expect("current task");
    let parsed: toml::Value = toml::from_str(&source).expect("task TOML");
    let generation = parsed["provenance"]["generations"]
        .as_array()
        .expect("generations")
        .iter()
        .find(|generation| {
            generation
                .get("prompt_path")
                .and_then(toml::Value::as_str)
                .is_some_and(|path| path.contains("module-prompts/GENERATOR-PROMPT.md"))
        })
        .expect("module generation");
    let original_path = generation["prompt_path"].as_str().expect("prompt path");
    let original_digest = generation["prompt_sha256"].as_str().expect("prompt digest");
    let path_line = format!("prompt_path = \"{original_path}\"");
    let digest_line = format!("prompt_sha256 = \"{original_digest}\"");
    assert_eq!(source.matches(&path_line).count(), 1);
    assert_eq!(source.matches(&digest_line).count(), 1);

    let missing_path = "missing-cited-prompt.md";
    let missing_path_line = format!("prompt_path = \"{missing_path}\"");
    let missing_digest_line = format!("prompt_sha256 = \"{ZERO}\"");
    let missing = source.replacen(&path_line, &missing_path_line, 1).replacen(
        &digest_line,
        &missing_digest_line,
        1,
    );

    let dir = temp_dir("citation-task");
    let record = dir.join("task.toml");
    std::fs::write(&record, &missing).expect("missing task");
    let output = run_lint(&record);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stdout.contains("accepted with unchecked citations (task)"));
    assert!(stdout.contains("citation.unchecked: missing-cited-prompt.md"));
    assert!(
        !stdout
            .lines()
            .any(|line| line.contains(": accepted (task)"))
    );

    let content = b"prompt citation beside this temporary record\n";
    let digest = xtask::util::sha256_hex(content);
    std::fs::write(dir.join(missing_path), content).expect("beside-record prompt");
    let matching = missing.replacen(
        &missing_digest_line,
        &format!("prompt_sha256 = \"{digest}\""),
        1,
    );
    std::fs::write(&record, matching).expect("matching task");
    let output = run_lint(&record);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stdout.contains("accepted with unchecked citations (task)"));
    assert!(!stdout.contains("citation.unchecked: missing-cited-prompt.md"));

    std::fs::write(dir.join(missing_path), b"tampered prompt\n").expect("tamper prompt");
    let output = run_lint(&record);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(stdout.contains("digest.mismatch"), "{stdout}");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn declared_pairs_and_registered_digest_maps_are_citations_but_inline_prompt_is_not() {
    let dir = temp_dir("citation-pairs");
    let record = dir.join("pairs.json");
    let inline = "this is prompt text, not a prompt path";
    let body = json!({
        "kind": "h4",
        "prompt": inline,
        "prompt_sha256": xtask::util::sha256_hex(inline.as_bytes()),
        "path": "missing-path.txt",
        "sha256": ZERO,
        "source": "missing-source.txt",
        "source_sha256": ZERO,
        "rules_path": "missing-rules.toml",
        "rules_digest": ZERO,
        "manifest_path": "missing-manifest.toml",
        "manifest_sha256": ZERO,
        "source_files_sha256": {"missing-source-map.rs": ZERO},
        "fixture_inputs_sha256": {"missing-fixture-input": ZERO},
        "generated_inputs_sha256": {"missing-generated-input": ZERO},
        "inputs_sha256": {"missing-input": ZERO},
        "unregistered_sha256": {"not-a-citation.txt": ZERO}
    });
    std::fs::write(&record, serde_json::to_vec(&body).expect("json")).expect("record");

    let report = xtask::record_lint::report_in(&record, Some("h4"), &[], None);
    for path in [
        "missing-path.txt",
        "missing-source.txt",
        "missing-rules.toml",
        "missing-manifest.toml",
        "missing-source-map.rs",
        "missing-fixture-input",
        "missing-generated-input",
        "missing-input",
    ] {
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains(&format!("citation.unchecked: {path}"))),
            "missing citation note for {path}: {:?}",
            report.notes
        );
    }
    assert!(!report.notes.iter().any(|note| note.contains(inline)));
    assert!(
        !report
            .notes
            .iter()
            .any(|note| note.contains("not-a-citation.txt"))
    );

    let mut malformed = body.clone();
    malformed["inputs_sha256"]["missing-input"] = json!("not-a-digest");
    std::fs::write(&record, serde_json::to_vec(&malformed).expect("json")).expect("record");
    let report = xtask::record_lint::report_in(&record, Some("h4"), &[], None);
    assert!(report.reasons.contains("digest.malformed"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn absolute_citations_use_the_subject_git_object_and_refuse_escape() {
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(root())
            .args(args)
            .output()
            .expect("git");
        assert!(output.status.success(), "{output:?}");
        output.stdout
    };
    let head = String::from_utf8(git(&["rev-parse", "HEAD"]))
        .expect("head")
        .trim()
        .to_owned();
    let cargo = git(&["cat-file", "blob", "HEAD:Cargo.toml"]);
    let digest = xtask::util::sha256_hex(&cargo);

    let dir = temp_dir("citation-absolute");
    let record = dir.join("absolute.json");
    let absolute = root().join("Cargo.toml");
    let body = json!({
        "kind": "h4",
        "artifact_identity": {"revision": head},
        "path": absolute.display().to_string(),
        "sha256": digest
    });
    std::fs::write(&record, serde_json::to_vec(&body).expect("json")).expect("record");
    let report = xtask::record_lint::report_in(&record, Some("h4"), &[], Some(root()));
    assert!(
        !report
            .notes
            .iter()
            .any(|note| note.contains(&absolute.display().to_string()))
    );
    assert!(!report.reasons.contains("digest.mismatch"));

    let outside = dir.join("outside.txt");
    std::fs::write(&outside, b"must not be read").expect("outside file");
    let outside_body = json!({
        "kind": "h4",
        "artifact_identity": {"revision": head},
        "path": outside.display().to_string(),
        "sha256": ZERO
    });
    std::fs::write(&record, serde_json::to_vec(&outside_body).expect("json"))
        .expect("outside record");
    let report = xtask::record_lint::report_in(&record, Some("h4"), &[], Some(root()));
    assert!(report.notes.iter().any(|note| {
        note.contains("citation.unchecked:") && note.contains(&outside.display().to_string())
    }));
    assert!(!report.reasons.contains("digest.mismatch"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let repo = dir.join("repo");
        let escape_target = dir.join("escape-target");
        std::fs::create_dir_all(&repo).expect("repo");
        std::fs::create_dir_all(&escape_target).expect("escape target");
        std::fs::write(escape_target.join("secret.txt"), b"must not be read").expect("secret");
        symlink(&escape_target, repo.join("escape")).expect("symlink");
        let escaped = repo.join("escape/secret.txt");
        let escaped_body = json!({
            "kind": "h4",
            "path": escaped.display().to_string(),
            "sha256": ZERO
        });
        std::fs::write(&record, serde_json::to_vec(&escaped_body).expect("json"))
            .expect("escaped record");
        let report = xtask::record_lint::report_in(&record, Some("h4"), &[], Some(&repo));
        assert!(report.notes.iter().any(|note| {
            note.contains("citation.unchecked:") && note.contains(&escaped.display().to_string())
        }));
        assert!(!report.reasons.contains("digest.mismatch"));
    }

    let _ = std::fs::remove_dir_all(dir);
}
