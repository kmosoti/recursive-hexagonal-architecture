use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xtask::graph::model::Target;
use xtask::util;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rha-modules-cli-{label}-{}-{nonce}",
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
        .expect("xtask has a workspace parent")
        .to_path_buf()
}

fn run_architecture(manifest: &Path, rules: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "architecture",
            "--manifest-path",
            manifest.to_str().expect("manifest path UTF-8"),
            "--rules",
            rules.to_str().expect("rules path UTF-8"),
            "--format",
            "json",
        ])
        .output()
        .expect("xtask architecture binary")
}

fn json_output(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "architecture output was not JSON: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_summary_matches_findings(report: &Value) {
    let findings = report["findings"].as_array().expect("findings array");
    let errors = findings
        .iter()
        .filter(|finding| finding["severity"] == "error")
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding["severity"] == "warning")
        .count();
    assert_eq!(report["summary"]["errors"], errors);
    assert_eq!(report["summary"]["warnings"], warnings);
    assert_eq!(
        report["summary"]["outcome"],
        if errors == 0 { "passed" } else { "failed" }
    );
}

fn assert_config_error(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = json_output(output);
    assert_eq!(report["summary"]["error_class"], "config_error");
    assert_eq!(report["summary"]["outcome"], "failed");
    assert_ne!(report["summary"]["outcome"], "passed");
    assert_ne!(report["summary"]["outcome"], "not_run");
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("file parent")).expect("file parent");
    fs::write(path, contents).expect("fixture file");
}

fn standalone_fixture() -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let temp = TempDir::new("standalone");
    let workspace = temp.path();
    let manifest = workspace.join("Cargo.toml");
    let rules = workspace.join("rha-crates.toml");
    let core = workspace.join("core");
    let helper = workspace.join("helper");
    let utility = workspace.join("utility");

    write(
        &manifest,
        r#"[workspace]
members = ["core", "helper"]
exclude = ["utility"]
resolver = "3"
"#,
    );
    write(
        &core.join("Cargo.toml"),
        r#"[package]
name = "core-pkg"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "core_custom"
path = "src/custom.rs"

[[bin]]
name = "runner"
path = "src/runner.rs"

[package.metadata.rha]
role = "core"
composite = "rules.toml"
"#,
    );
    write(
        &core.join("src/custom.rs"),
        r#"#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]
pub mod constraints;
pub mod ordering;
"#,
    );
    write(
        &core.join("src/runner.rs"),
        r#"#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]
fn main() {}
"#,
    );
    write(&core.join("src/constraints.rs"), "pub struct Rule;\n");
    write(
        &core.join("src/ordering.rs"),
        "pub fn value() -> crate::constraints::Rule { crate::constraints::Rule }\n",
    );
    fs::copy(
        root().join("xtask/templates/core-clippy.toml"),
        core.join("clippy.toml"),
    )
    .expect("core Clippy template");
    write(
        &core.join("rules.toml"),
        r#"[components]
constraints = "core_custom::constraints"
ordering = "core_custom::ordering"

[allow]
constraints = []
ordering = ["constraints"]

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#,
    );

    write(
        &helper.join("Cargo.toml"),
        r#"[package]
name = "helper"
version = "0.1.0"
edition = "2024"
publish = false

[package.metadata.rha]
role = "app"

[dependencies]
aliased = { package = "utility", path = "../utility" }
"#,
    );
    write(
        &helper.join("src/lib.rs"),
        "pub fn use_utility() -> u8 { aliased::value() }\n",
    );
    write(
        &utility.join("Cargo.toml"),
        r#"[package]
name = "utility"
version = "0.1.0"
edition = "2024"
publish = false
"#,
    );
    write(&utility.join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n");

    write(
        &rules,
        r#"schema_version = 1

[classification]
adapter_prefix = "adapter-"
app_prefix = "app-"
tools = []
harness = []

[core]
allow = []
dev_allow = []
allow_build_scripts = false

[adapters]
require_port_owner_dependency = true
foreign_core_dependency = "warn"

[transitive]
enabled = false
"#,
    );

    (temp, manifest, rules, core)
}

#[test]
fn architecture_cli_checks_live_site_and_preserves_report_contract() {
    let workspace = root();
    let output = run_architecture(
        &workspace.join("Cargo.toml"),
        &workspace.join("rha-crates.toml"),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = json_output(&output);
    assert_summary_matches_findings(&report);
    assert!(report["test_edges"].is_array());
    assert!(report["module_edges"].is_array());
    let site = report["module_checks"]
        .as_array()
        .expect("module_checks array")
        .iter()
        .find(|check| check["crate"] == "site")
        .expect("site module check");
    assert_eq!(site["required"], true);
    assert_eq!(site["outcome"], "passed");
    assert!(site["test_edges"].is_array() || report["test_edges"].is_array());
    assert!(site["module_edges"].is_array() || report["module_edges"].is_array());

    let source_files = site["source_files_sha256"]
        .as_object()
        .expect("site source_files_sha256");
    for relative in [
        "xtask/tests/support/registered_package.rs",
        "xtask/tests/support/registered_package_embedded.rs",
    ] {
        let path = fs::canonicalize(workspace.join(relative)).expect("canonical support source");
        let key = path.display().to_string();
        let actual = source_files
            .get(&key)
            .unwrap_or_else(|| panic!("site source digest missing for {key}"));
        assert_eq!(
            actual,
            &serde_json::json!(util::sha256_file(&path).expect("support source digest"))
        );
    }
    assert!(
        source_files.keys().all(|path| path.ends_with(".rs")),
        "embedded corpus payloads must not be labelled as Rust modules: {source_files:?}"
    );

    let test_edges = report["test_edges"].as_array().expect("test_edges array");
    let module_edges = report["module_edges"].as_array().expect("module_edges array");
    let helper_prefix = "assembly::transclusion_tests::registered_package";
    let helper_edges = test_edges
        .iter()
        .filter(|edge| {
            edge["from"].as_str().is_some_and(|from| {
                from == helper_prefix || from.starts_with(&format!("{helper_prefix}::"))
            })
        })
        .collect::<Vec<_>>();
    assert!(
        !helper_edges.is_empty(),
        "shared helper edges were not reported as test edges"
    );
    for edge in helper_edges {
        assert_eq!(edge["severity"], "note");
        assert!(!module_edges.iter().any(|candidate| {
            candidate["from"] == edge["from"] && candidate["to"] == edge["to"]
        }));
    }
}

#[test]
fn standalone_metadata_and_cli_cover_identity_aliases_and_module_failures() {
    let (_temp, manifest, rules, core) = standalone_fixture();
    let workspace = manifest.parent().expect("standalone workspace");

    let valid = run_architecture(&manifest, &rules);
    assert_eq!(
        valid.status.code(),
        Some(0),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&valid.stderr),
        String::from_utf8_lossy(&valid.stdout)
    );
    let valid_report = json_output(&valid);
    assert_summary_matches_findings(&valid_report);
    let module_check = valid_report["module_checks"]
        .as_array()
        .expect("module_checks array")
        .iter()
        .find(|check| check["crate"] == "core-pkg")
        .expect("core module check");
    assert_eq!(module_check["required"], true);
    assert_eq!(module_check["outcome"], "passed");
    assert_eq!(
        module_check["source"],
        core.join("src/custom.rs").display().to_string()
    );
    assert_eq!(
        module_check["rules_path"],
        core.join("rules.toml").display().to_string()
    );

    let graph = xtask::metadata::load(workspace, Some(&manifest)).expect("standalone metadata");
    let core_node = graph.crate_named("core-pkg").expect("core package");
    let composite = core_node.composite.as_ref().expect("core composite");
    assert_eq!(composite.crate_name, "core_custom");
    assert_eq!(composite.edition, "2021");
    assert_eq!(composite.source, core.join("src/custom.rs"));
    assert_eq!(composite.rules, "rules.toml");
    assert_eq!(
        core_node.source_roots,
        vec![core.join("src/custom.rs"), core.join("src/runner.rs")]
    );
    let aliased = graph
        .edges
        .iter()
        .find(|edge| edge.from == "helper" && edge.to.name() == "utility")
        .expect("aliased helper dependency");
    assert!(matches!(aliased.to, Target::External { .. }));
    assert_eq!(aliased.rename.as_deref(), Some("aliased"));

    let bin_root = core.join("src/runner.rs");
    write(&bin_root, "fn main() {}\n");
    let missing_bin = run_architecture(&manifest, &rules);
    assert_eq!(
        missing_bin.status.code(),
        Some(1),
        "stderr={}",
        String::from_utf8_lossy(&missing_bin.stderr)
    );
    let missing_bin_report = json_output(&missing_bin);
    assert_summary_matches_findings(&missing_bin_report);
    assert!(
        missing_bin_report["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| {
                finding["rule"] == "effect.core_clippy_template"
                    && finding["manifest_path"] == bin_root.display().to_string()
            })
    );
    write(
        &bin_root,
        r#"#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]
fn main() {}
"#,
    );

    let custom_root = core.join("src/custom.rs");
    write(&custom_root, "pub mod constraints;\npub mod ordering;\n");
    let missing_custom = run_architecture(&manifest, &rules);
    assert_eq!(
        missing_custom.status.code(),
        Some(1),
        "stderr={}",
        String::from_utf8_lossy(&missing_custom.stderr)
    );
    let missing_custom_report = json_output(&missing_custom);
    assert_summary_matches_findings(&missing_custom_report);
    assert!(
        missing_custom_report["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| {
                finding["rule"] == "effect.core_clippy_template"
                    && finding["manifest_path"] == custom_root.display().to_string()
            })
    );
    write(
        &custom_root,
        r#"#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]
pub mod constraints;
pub mod ordering;
"#,
    );

    write(
        &core.join("src/constraints.rs"),
        "pub struct Rule;\npub fn illegal() { let _ = crate::ordering::value(); }\n",
    );
    let invalid = run_architecture(&manifest, &rules);
    assert_eq!(
        invalid.status.code(),
        Some(1),
        "stderr={}",
        String::from_utf8_lossy(&invalid.stderr)
    );
    let invalid_report = json_output(&invalid);
    assert_summary_matches_findings(&invalid_report);
    let failed_check = invalid_report["module_checks"]
        .as_array()
        .expect("module_checks array")
        .iter()
        .find(|check| check["crate"] == "core-pkg")
        .expect("failed core module check");
    assert_eq!(failed_check["outcome"], "failed");
    assert!(
        invalid_report["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| {
                finding["rule"] == "modules.cycle"
                    && finding["path"]
                        == serde_json::json!(["constraints", "ordering", "constraints"])
                    && finding["witness"]["source"] == "core_custom::constraints"
            })
    );

    fs::remove_file(core.join("rules.toml")).expect("remove declared module rules");
    assert_config_error(&run_architecture(&manifest, &rules));

    write(&core.join("rules.toml"), "[components]\nconstraints = ");
    assert_config_error(&run_architecture(&manifest, &rules));
}
