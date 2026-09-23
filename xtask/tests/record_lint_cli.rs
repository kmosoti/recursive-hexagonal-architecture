//! `cargo xtask rha lint` with no operand is a usage error, never a vacuous
//! pass (PR 17, automated review thread).

#[test]
fn a_lint_run_without_records_is_a_usage_error() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["rha", "lint"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

/// A cited file that is not beside the record is checked at the record's
/// subject in git, so a tampered digest in a repository record is caught
/// (Opus 5.5 review of pull request 17, finding 3). The subject is HEAD,
/// which a shallow CI checkout also has.
#[test]
fn a_cited_digest_is_checked_at_the_records_subject() {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git");
        assert!(out.status.success(), "{out:?}");
        out.stdout
    };
    let head = String::from_utf8(git(&["rev-parse", "HEAD"])).expect("utf8");
    let real = xtask::util::sha256_hex(&git(&["cat-file", "blob", "HEAD:Cargo.toml"]));
    let dir = std::env::temp_dir().join(format!("rha-lint-subject-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp");
    let record = |digest: &str| {
        let path = dir.join(format!("{}.json", &digest[..8]));
        let body = serde_json::json!({
            "artifact_identity": {"revision": head.trim()},
            "verification_identity": {"instruction_sources": [{"path": "Cargo.toml", "sha256": digest}]},
        });
        std::fs::write(&path, body.to_string()).expect("write");
        path
    };
    let lint = |p: &std::path::Path| xtask::record_lint::lint_in(p, "evidence", &[], Some(root));
    assert!(!lint(&record(&real)).contains("digest.mismatch"));
    assert!(lint(&record(&"0".repeat(64))).contains("digest.mismatch"));
}
