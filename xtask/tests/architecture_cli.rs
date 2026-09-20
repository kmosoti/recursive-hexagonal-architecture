//! End-to-end architecture command contracts using tiny dependency-free
//! workspaces. The fixtures live in the system temporary directory and are
//! removed by each test.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rha-architecture-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).expect("temporary workspace");
    path
}

fn rules(path: &Path, schema_version: u32) -> PathBuf {
    let file = path.join("rha-crates.toml");
    std::fs::write(
        &file,
        format!(
            "schema_version = {schema_version}\n[classification]\nadapter_prefix = \"adapter-\"\napp_prefix = \"app-\"\ntools = []\nharness = []\n"
        ),
    )
    .expect("rules file");
    file
}

fn workspace(path: &Path, name: &str, metadata: &str) -> PathBuf {
    std::fs::write(
        path.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crate\"]\nresolver = \"3\"\n",
    )
    .expect("workspace manifest");
    let crate_dir = path.join("crate");
    std::fs::create_dir_all(crate_dir.join("src")).expect("crate directory");
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n{metadata}"
        ),
    )
    .expect("crate manifest");
    std::fs::write(crate_dir.join("src/lib.rs"), "pub fn value() {}\n").expect("crate source");
    crate_dir.join("Cargo.toml")
}

fn run(manifest: &Path, rules: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "architecture",
            "--manifest-path",
            manifest.to_str().expect("manifest utf8"),
            "--rules",
            rules.to_str().expect("rules utf8"),
            "--format",
            "json",
        ])
        .output()
        .expect("xtask binary")
}

fn assert_tool_revision(value: &Value) {
    let revision = value.as_str().expect("tool git_rev string");
    assert!(
        revision == "unknown"
            || (matches!(revision.len(), 40 | 64)
                && revision.bytes().all(|byte| byte.is_ascii_hexdigit())),
        "git_rev must be a full hash or unknown: {revision}"
    );
}

#[test]
fn malformed_metadata_is_configuration_error_and_absence_is_valid() {
    let malformed_root = temp_dir("malformed");
    let malformed = workspace(
        &malformed_root,
        "adapter-bad",
        "[package.metadata.rha]\nrole = \"adapter\"\nimplements = \"core-a::Port\"\n",
    );
    let malformed_rules = rules(&malformed_root, 1);
    let failed = run(&malformed, &malformed_rules);
    assert_eq!(failed.status.code(), Some(2));
    let failed_json: Value = serde_json::from_slice(&failed.stdout).expect("failure JSON");
    assert_eq!(failed_json["summary"]["error_class"], "config_error");
    assert!(failed_json["subject"]["workspace_root"].is_null());
    assert!(
        failed_json["summary"]["reason"]
            .as_str()
            .expect("failure reason")
            .contains("adapter-bad")
    );
    assert!(
        failed_json["summary"]["reason"]
            .as_str()
            .expect("failure reason")
            .contains("implements")
    );
    assert!(failed_json["tool"]["git_rev"].is_string());
    assert_tool_revision(&failed_json["tool"]["git_rev"]);
    assert!(
        failed_json["tool"]["git_dirty"].is_boolean() || failed_json["tool"]["git_dirty"].is_null()
    );
    std::fs::remove_dir_all(malformed_root).expect("clean malformed workspace");

    let valid_root = temp_dir("absent");
    let valid = workspace(&valid_root, "adapter-ok", "");
    let valid_rules = rules(&valid_root, 1);
    let passed = run(&valid, &valid_rules);
    assert_eq!(passed.status.code(), Some(0));
    let passed_json: Value = serde_json::from_slice(&passed.stdout).expect("success JSON");
    assert_eq!(passed_json["summary"]["outcome"], "passed");
    assert_tool_revision(&passed_json["tool"]["git_rev"]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent");
    assert_eq!(
        passed_json["tool"],
        xtask::graph::report::tool_identity(root)
    );
    assert_eq!(failed_json["tool"], passed_json["tool"]);
    std::fs::remove_dir_all(valid_root).expect("clean valid workspace");
}

#[test]
fn unsupported_schema_fails_before_loading_a_missing_manifest() {
    let root = temp_dir("schema");
    let rules_path = rules(&root, 2);
    let missing = root.join("does-not-exist/Cargo.toml");
    let output = run(&missing, &rules_path);
    assert_eq!(output.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&output.stdout).expect("failure JSON");
    assert_eq!(json["summary"]["error_class"], "config_error");
    assert!(
        json["summary"]["reason"]
            .as_str()
            .expect("failure reason")
            .contains("schema_version")
    );
    std::fs::remove_dir_all(root).expect("clean schema workspace");
}

#[test]
fn same_name_external_owner_does_not_satisfy_a_port_claim() {
    let root = temp_dir("same-name");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"core-a\", \"adapter-x\"]\nresolver = \"3\"\n",
    )
    .expect("workspace manifest");
    let core = root.join("core-a");
    let adapter = root.join("adapter-x");
    let external = root.with_file_name(format!(
        "{}-external-core-a",
        root.file_name()
            .expect("temporary workspace name")
            .to_string_lossy()
    ));
    for dir in [&core, &adapter, &external] {
        std::fs::create_dir_all(dir.join("src")).expect("fixture crate");
    }
    std::fs::write(
        core.join("Cargo.toml"),
        "[package]\nname = \"core-a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[package.metadata.rha]\nrole = \"core\"\n",
    )
    .expect("core manifest");
    std::fs::write(
        core.join("src/lib.rs"),
        "#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]\n",
    )
    .expect("core source");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/core-clippy.toml"),
        core.join("clippy.toml"),
    )
    .expect("core clippy template");
    std::fs::write(
        adapter.join("Cargo.toml"),
        format!(
            "[package]\nname = \"adapter-x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[package.metadata.rha]\nrole = \"adapter\"\nimplements = [\"core-a::Port\"]\n\n[dependencies]\ncore-a = {{ package = \"core-a\", path = \"{}\" }}\n",
            external.display()
        ),
    )
    .expect("adapter manifest");
    std::fs::write(adapter.join("src/lib.rs"), "pub fn adapter() {}\n").expect("adapter source");
    std::fs::write(
        external.join("Cargo.toml"),
        "[package]\nname = \"core-a\"\nversion = \"0.2.0\"\nedition = \"2024\"\n",
    )
    .expect("external manifest");
    std::fs::write(external.join("src/lib.rs"), "pub fn external() {}\n").expect("external source");
    let rules_path = rules(&root, 1);
    let output = run(&adapter.join("Cargo.toml"), &rules_path);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("success JSON");
    assert_eq!(json["summary"]["errors"], 1);
    assert!(
        json["findings"]
            .as_array()
            .expect("findings array")
            .iter()
            .any(|finding| finding["rule"] == "adapter.missing_port_owner")
    );

    // The same adapter claim is valid when the dependency points at the
    // actual workspace member. This control catches a false positive from
    // requiring a name-only match or from losing normal member edges.
    std::fs::write(
        adapter.join("Cargo.toml"),
        "[package]\nname = \"adapter-x\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[package.metadata.rha]\nrole = \"adapter\"\nimplements = [\"core-a::Port\"]\n\n[dependencies]\ncore-a = { package = \"core-a\", path = \"../core-a\" }\n",
    )
    .expect("member dependency manifest");
    let control = run(&adapter.join("Cargo.toml"), &rules_path);
    assert_eq!(
        control.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&control.stderr)
    );
    std::fs::remove_dir_all(root).expect("clean same-name workspace");
    std::fs::remove_dir_all(external).expect("clean external workspace");
}
