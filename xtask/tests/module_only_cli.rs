use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rha-module-only-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask workspace root")
        .to_path_buf()
}

fn fixture(id: &str) -> (PathBuf, PathBuf) {
    let directory = root().join("xtask/tests/corpus/module/headline").join(id);
    (
        directory.join("Cargo.toml"),
        directory.join("rha-modules.toml"),
    )
}

fn run(manifest: &Path, rules: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command.args([
        "architecture",
        "--manifest-path",
        manifest.to_str().expect("manifest path"),
        "--module-rules",
        rules.to_str().expect("rules path"),
        "--format",
        "json",
    ]);
    command.args(extra);
    command.output().expect("xtask binary")
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "expected JSON stdout: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("file parent")).expect("file parent");
    fs::write(path, contents).expect("fixture file");
}

#[test]
fn frozen_module_fixtures_use_module_only_reports() {
    let (m01_manifest, m01_rules) = fixture("M01");
    let m01 = run(&m01_manifest, &m01_rules, &[]);
    assert_eq!(m01.status.code(), Some(1));
    let m01_report = json(&m01);
    assert_eq!(m01_report["subject"]["mode"], "module_only");
    assert_eq!(m01_report["subject"]["metadata_mode"], "no_deps");
    assert_eq!(m01_report["module_checks"][0]["required"], true);
    assert_eq!(m01_report["module_checks"][0]["crate"], "fixture_m01");
    assert_eq!(
        m01_report["findings"]
            .as_array()
            .expect("M01 findings")
            .iter()
            .find(|finding| finding["rule"] == "modules.cycle")
            .expect("M01 cycle")["path"],
        json!(["constraints", "ordering", "constraints"])
    );

    let (m11_manifest, m11_rules) = fixture("M11");
    let m11 = run(&m11_manifest, &m11_rules, &[]);
    assert_eq!(m11.status.code(), Some(0));
    let m11_report = json(&m11);
    assert_eq!(m11_report["summary"]["errors"], 0);
    assert!(
        m11_report["findings"]
            .as_array()
            .expect("M11 findings")
            .is_empty()
    );
    assert_eq!(m11_report["test_edges"][0]["severity"], "note");
    assert_eq!(m11_report["test_edges"][0]["from"], "constraints::tests");
    assert_eq!(m11_report["test_edges"][0]["to"], "ordering");

    let (em_manifest, em_rules) = fixture("EM-M01");
    let em = run(&em_manifest, &em_rules, &[]);
    assert_eq!(em.status.code(), Some(0));
    let em_report = json(&em);
    assert_eq!(em_report["summary"]["errors"], 0);
    assert!(
        em_report["findings"]
            .as_array()
            .expect("EM-M01 findings")
            .is_empty()
    );
    let limitations = em_report["limitations"].as_array().expect("limitations");
    assert!(limitations.iter().any(|limitation| {
        limitation["code"] == "source_approximation" && limitation["crate"] == "fixture_em_m01"
    }));
    assert!(
        limitations
            .iter()
            .any(|limitation| limitation["code"] == "macro_definition")
    );
    assert!(
        limitations
            .iter()
            .any(|limitation| limitation["code"] == "macro_expansion")
    );
}

#[test]
fn module_only_rejects_multi_package_workspaces_and_invalid_cli_combinations() {
    let (fixture_manifest, fixture_rules) = fixture("M01");

    let full_repo = run(&root().join("Cargo.toml"), &fixture_rules, &[]);
    assert_eq!(full_repo.status.code(), Some(2));
    let full_repo_report = json(&full_repo);
    assert_eq!(full_repo_report["summary"]["error_class"], "config_error");
    assert_eq!(full_repo_report["subject"]["mode"], "module_only");
    assert!(
        full_repo_report["summary"]["reason"]
            .as_str()
            .expect("multi-workspace failure reason")
            .contains("exactly one workspace package")
    );

    let missing_manifest = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "architecture",
            "--module-rules",
            fixture_rules.to_str().expect("rules path"),
        ])
        .output()
        .expect("xtask binary");
    assert_eq!(missing_manifest.status.code(), Some(2));

    let conflicting_rules = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "architecture",
            "--manifest-path",
            fixture_manifest.to_str().expect("manifest path"),
            "--module-rules",
            fixture_rules.to_str().expect("module rules path"),
            "--rules",
            fixture_rules.to_str().expect("rules path"),
        ])
        .output()
        .expect("xtask binary");
    assert_eq!(conflicting_rules.status.code(), Some(2));

    let conflicting_transitive = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "architecture",
            "--manifest-path",
            fixture_manifest.to_str().expect("manifest path"),
            "--module-rules",
            fixture_rules.to_str().expect("module rules path"),
            "--transitive",
        ])
        .output()
        .expect("xtask binary");
    assert_eq!(conflicting_transitive.status.code(), Some(2));
}

#[test]
fn module_only_rejects_a_single_member_virtual_workspace_manifest() {
    let temp = TempDir::new("virtual-workspace");
    let manifest = temp.path().join("Cargo.toml");
    let member = temp.path().join("member");
    let rules = temp.path().join("rha-modules.toml");

    write(
        &manifest,
        r#"[workspace]
members = ["member"]
"#,
    );
    write(
        &member.join("Cargo.toml"),
        r#"[package]
name = "virtual-member"
version = "0.0.0"
edition = "2021"

[lib]
name = "virtual_member"
path = "src/lib.rs"
"#,
    );
    write(&member.join("src/lib.rs"), "pub mod component;\n");
    write(&member.join("src/component.rs"), "pub fn call() {}\n");
    write(
        &rules,
        r#"[components]
component = "virtual_member::component"

[allow]
component = []

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );

    let output = run(&manifest, &rules, &[]);
    assert_eq!(output.status.code(), Some(2));
    let report = json(&output);
    assert_eq!(report["summary"]["error_class"], "config_error");
    assert!(
        report["summary"]["reason"]
            .as_str()
            .expect("virtual workspace failure reason")
            .contains("virtual workspace")
    );
}

#[test]
fn module_only_rejects_missing_rules_and_malformed_source() {
    let (manifest, rules) = fixture("M01");
    let missing_rules = run(
        &manifest,
        &manifest.with_file_name("missing-rha-modules.toml"),
        &[],
    );
    assert_eq!(missing_rules.status.code(), Some(2));
    assert_eq!(
        json(&missing_rules)["summary"]["error_class"],
        "config_error"
    );

    let temp = TempDir::new("malformed-source");
    let malformed_manifest = temp.path().join("Cargo.toml");
    let malformed_rules = temp.path().join("rha-modules.toml");
    write(
        &malformed_manifest,
        r#"[package]
name = "malformed"
version = "0.0.0"
edition = "2021"
[workspace]
"#,
    );
    write(
        &temp.path().join("src/lib.rs"),
        "pub mod { this is not Rust;\n",
    );
    fs::copy(&rules, &malformed_rules).expect("module rules");
    let malformed = run(&malformed_manifest, &malformed_rules, &[]);
    assert_eq!(malformed.status.code(), Some(2));
    assert_eq!(json(&malformed)["summary"]["error_class"], "config_error");
}

#[test]
fn module_only_uses_library_identity_and_external_dependency_aliases() {
    let temp = TempDir::new("identity");
    let manifest = temp.path().join("Cargo.toml");
    let rules = temp.path().join("rha-modules.toml");
    write(
        &manifest,
        r#"[package]
name = "package-name"
version = "0.0.0"
edition = "2021"

[workspace]
exclude = ["external-dep"]

[lib]
name = "custom_lib"
path = "src/custom.rs"

[dependencies]
renamed_dep = { package = "external-dep", path = "external-dep" }
"#,
    );
    write(
        &temp.path().join("external-dep/Cargo.toml"),
        r#"[package]
name = "external-dep"
version = "0.0.0"
edition = "2021"
"#,
    );
    write(
        &temp.path().join("external-dep/src/lib.rs"),
        "pub fn value() {}\n",
    );
    write(&temp.path().join("src/custom.rs"), "pub mod component;\n");
    write(
        &temp.path().join("src/component.rs"),
        "pub fn call() { renamed_dep::value(); }\n",
    );
    write(
        &rules,
        r#"[components]
component = "custom_lib::component"

[allow]
component = []

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );

    let output = run(&manifest, &rules, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let report = json(&output);
    assert_eq!(report["subject"]["package_name"], "package-name");
    assert_eq!(report["subject"]["crate_name"], "custom_lib");
    assert_eq!(report["module_checks"][0]["package"], "package-name");
    assert_eq!(report["module_checks"][0]["crate"], "custom_lib");
    let source_files = report["subject"]["source_files_sha256"]
        .as_object()
        .expect("source files");
    let custom_root =
        fs::canonicalize(temp.path().join("src/custom.rs")).expect("custom source path");
    let child = fs::canonicalize(temp.path().join("src/component.rs")).expect("child source path");
    assert!(source_files.contains_key(&custom_root.display().to_string()));
    assert!(source_files.contains_key(&child.display().to_string()));
    assert_eq!(source_files.len(), 2);
    assert_eq!(
        report["module_checks"][0]["source_files_sha256"],
        report["subject"]["source_files_sha256"]
    );
    let original_root_digest = report["subject"]["source_sha256"].clone();
    let original_child_digest = source_files[&child.display().to_string()].clone();
    write(
        &temp.path().join("src/component.rs"),
        "pub fn call() { renamed_dep::value(); }\n\n",
    );
    let tampered = run(&manifest, &rules, &[]);
    assert_eq!(tampered.status.code(), Some(0));
    let tampered_report = json(&tampered);
    assert_eq!(
        tampered_report["subject"]["source_sha256"],
        original_root_digest
    );
    assert_ne!(
        tampered_report["subject"]["source_files_sha256"][&child.display().to_string()],
        original_child_digest
    );
    assert!(
        report["module_edges"]
            .as_array()
            .expect("module edges")
            .is_empty()
    );
}
