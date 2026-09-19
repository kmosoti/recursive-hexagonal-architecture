//! The Clippy corpus of CHG-001, rerun on every L0 run so a toolchain change
//! re-verifies the configuration discovery rule recorded in ADR-0002.
//!
//! Each fixture is its own workspace, excluded from the root workspace, and is
//! built into a dedicated directory under the root `target/`, cleaned first so
//! no cached result stands in for a fresh one.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap_or(manifest_dir).to_path_buf()
}

fn corpus(fixture: &str) -> PathBuf {
    root().join("xtask/tests/corpus/clippy").join(fixture)
}

fn toml_table(path: &Path) -> toml::Table {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

/// Runs `cargo clippy ARGS` in `fixture`; returns (exit status, stderr).
fn clippy(fixture: &str, target: &Path, args: &[&str]) -> (i32, String) {
    let output = xtask::util::command("cargo")
        .arg("clippy")
        .args(args)
        .current_dir(corpus(fixture))
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_TERM_COLOR", "never")
        .env_remove("CLIPPY_CONF_DIR")
        .output()
        .unwrap_or_else(|e| panic!("running cargo clippy in {fixture}: {e}"));
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn fresh_target(fixture: &str) -> PathBuf {
    let dir = root().join("target/clippy-corpus").join(fixture);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

const UNWRAP_FINDING: &str = "used `unwrap()` on a `Result` value";

#[test]
fn template_repeats_every_root_setting() {
    let root_settings = toml_table(&root().join("clippy.toml"));
    let template = toml_table(&root().join("xtask/templates/core-clippy.toml"));
    for (key, value) in &root_settings {
        assert_eq!(
            template.get(key),
            Some(value),
            "template must repeat root setting {key}"
        );
    }
}

#[test]
fn fixtures_copy_the_root_lint_tables_and_root_clippy_file() {
    let root_manifest = toml_table(&root().join("Cargo.toml"));
    let root_lints = &root_manifest["workspace"]["lints"];
    let root_clippy = std::fs::read(root().join("clippy.toml")).unwrap();
    for fixture in ["discovery", "core-seeded"] {
        let manifest = toml_table(&corpus(fixture).join("Cargo.toml"));
        assert_eq!(
            &manifest["workspace"]["lints"], root_lints,
            "{fixture} lint tables drifted"
        );
        let fixture_clippy = std::fs::read(corpus(fixture).join("clippy.toml")).unwrap();
        assert_eq!(
            fixture_clippy, root_clippy,
            "{fixture}/clippy.toml drifted from the root file"
        );
    }
}

#[test]
fn discovery_is_per_crate_and_the_nearest_file_is_not_merged() {
    let target = fresh_target("discovery");

    // Experiment 2: the adapter makes the seeded call but has no local file.
    let (status, stderr) = clippy("discovery", &target, &["-p", "adapter-x"]);
    assert_eq!(status, 0, "{stderr}");
    assert!(!stderr.contains("disallowed"), "{stderr}");

    // Experiment 3: core-a's local file omits the root allowance.
    let (status, stderr) = clippy("discovery", &target, &["-p", "core-a", "--all-targets"]);
    assert_eq!(status, 0, "{stderr}");
    assert!(
        stderr.contains(UNWRAP_FINDING),
        "unwrap_used must fire: {stderr}"
    );

    // Control: the root allowance applies where the root file is the nearest.
    let (status, stderr) = clippy("discovery", &target, &["-p", "adapter-x", "--all-targets"]);
    assert_eq!(status, 0, "{stderr}");
    assert!(!stderr.contains(UNWRAP_FINDING), "{stderr}");
}

#[test]
fn the_template_denies_every_entry_and_keeps_test_allowances() {
    let target = fresh_target("core-seeded");

    // Experiment 1: the seeded call fails the build.
    let (status, stderr) = clippy("core-seeded", &target, &["-p", "core-a"]);
    assert_ne!(status, 0, "{stderr}");
    assert!(stderr.contains("clippy::disallowed_methods"), "{stderr}");
    assert!(stderr.contains("`std::time::SystemTime::now`"), "{stderr}");

    // Experiment 3b: the template's repeated allowance covers test unwrap();
    // core-b has nothing on the deny list, so a clean exit is decisive.
    let (status, stderr) = clippy("core-seeded", &target, &["-p", "core-b", "--all-targets"]);
    assert_eq!(status, 0, "{stderr}");
    assert!(!stderr.contains(UNWRAP_FINDING), "{stderr}");

    // Experiment 4: every path in the template is reported as disallowed.
    let (status, stderr) = clippy("core-seeded", &target, &["-p", "core-every"]);
    assert_ne!(status, 0, "{stderr}");
    let template = toml_table(&root().join("xtask/templates/core-clippy.toml"));
    for list in [
        "disallowed-methods",
        "disallowed-types",
        "disallowed-macros",
    ] {
        for entry in template[list].as_array().unwrap() {
            let path = entry["path"].as_str().unwrap();
            let fired = stderr
                .lines()
                .any(|l| l.contains("use of a disallowed") && l.contains(&format!("`{path}`")));
            assert!(fired, "{list} entry {path} did not fire:\n{stderr}");
        }
    }
}
