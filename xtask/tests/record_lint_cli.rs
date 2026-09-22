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
