//! `cargo metadata` as the one source of the crate graph (plan §5).
//!
//! The command is `cargo metadata --format-version 1 --no-deps --offline`,
//! with `--manifest-path` when an external workspace is checked. `--no-deps`
//! keeps the run fast and offline, and limits what can be seen to direct
//! declared edges — which is the §4.1 transitive hole, registered as corpus
//! case EM-C01 rather than hidden.
//!
//! Every field a rule needs comes from this output. Plan §8 W3 is explicit
//! that renamed, optional, target-conditional and inherited dependencies are
//! resolved through metadata fields and never by parsing `Cargo.toml` text,
//! because each of those four spellings hides the same edge differently
//! (corpus C15 to C18). `cargo metadata` has already resolved all four: a
//! workspace-inherited dependency appears as an ordinary entry, and a renamed
//! one reports the package in `name` with the key in `rename`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Context as _, Error, Result};
use crate::graph::model::{CrateGraph, CrateNode, DepKind, Edge, MetadataMode, PortRef, Target};

/// The subset of `cargo metadata --format-version 1` that a rule needs.
#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    #[serde(default)]
    dependencies: Vec<Dependency>,
    /// `[package.metadata]`, where `rha.role` and `rha.implements` live.
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    /// The build script target, when the package has one.
    #[serde(default)]
    targets: Vec<PackageTarget>,
}

#[derive(Debug, Deserialize)]
struct PackageTarget {
    #[serde(default)]
    kind: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    /// The PACKAGE name, even when the dependency is renamed.
    name: String,
    /// `null` for a normal dependency; otherwise `"dev"` or `"build"`.
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    optional: bool,
    /// The `cfg(...)` of a target-conditional dependency.
    #[serde(default)]
    target: Option<String>,
    /// The key in `Cargo.toml` when it differs from the package name.
    #[serde(default)]
    rename: Option<String>,
}

/// What `[package.metadata.rha]` may declare.
#[derive(Debug, Default, Deserialize)]
struct RhaMetadata {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    implements: Vec<String>,
}

/// Runs `cargo metadata` and builds the graph.
///
/// # Errors
/// Fails when cargo cannot be run, exits nonzero, or writes output this
/// cannot parse. Every one of those is an environment failure, reported with
/// exit code 3 and never as a pass (plan §5).
pub fn load(root: &Path, manifest_path: Option<&Path>) -> Result<CrateGraph> {
    let mut command = crate::util::command("cargo");
    command
        .current_dir(root)
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--offline");
    if let Some(path) = manifest_path {
        command.arg("--manifest-path").arg(path);
    }
    let output = command
        .output()
        .context(|| "running `cargo metadata`".to_owned())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(format!(
            "`cargo metadata` exited {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        )));
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .context(|| "parsing `cargo metadata` output".to_owned())?;
    Ok(build(&metadata))
}

/// Builds the graph from parsed metadata. Split out so tests can drive it
/// with recorded output instead of running cargo.
fn build(metadata: &Metadata) -> CrateGraph {
    let members: Vec<&Package> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .collect();
    let member_names: Vec<&str> = members.iter().map(|p| p.name.as_str()).collect();

    let mut crates = Vec::with_capacity(members.len());
    let mut edges = Vec::new();
    for package in &members {
        let rha = rha_metadata(package.metadata.as_ref());
        crates.push(CrateNode {
            name: package.name.clone(),
            manifest_path: package.manifest_path.clone(),
            // Classification happens in graph::classify; the graph only
            // carries what was declared.
            role: None,
            role_source: None,
            declared_role: rha.role.clone(),
            implements: rha.implements.iter().map(|p| PortRef::parse(p)).collect(),
            has_build_script: package
                .targets
                .iter()
                .any(|t| t.kind.iter().any(|k| k == "custom-build")),
        });
        for dep in &package.dependencies {
            let to = if member_names.contains(&dep.name.as_str()) {
                Target::Member {
                    name: dep.name.clone(),
                }
            } else {
                Target::External {
                    name: dep.name.clone(),
                }
            };
            edges.push(Edge {
                from: package.name.clone(),
                to,
                kind: DepKind::parse(dep.kind.as_deref()),
                optional: dep.optional,
                target_cfg: dep.target.clone(),
                rename: dep.rename.clone(),
            });
        }
    }
    CrateGraph {
        workspace_root: metadata.workspace_root.clone(),
        crates,
        edges,
        mode: MetadataMode::NoDeps,
    }
}

/// Reads `[package.metadata.rha]`, tolerating its absence. A malformed table
/// is treated as absent here; the crate is then unclassified, which
/// `class.unclassified` reports, rather than silently defaulting to a role.
fn rha_metadata(metadata: Option<&serde_json::Value>) -> RhaMetadata {
    metadata
        .and_then(|m| m.get("rha"))
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Metadata in the shape cargo writes, with the four dependency spellings
    /// of corpus cases C15 to C18 already resolved the way cargo resolves
    /// them. This is the point of reading metadata instead of Cargo.toml: the
    /// rename arrives as `name` + `rename`, the inherited dependency arrives
    /// as an ordinary entry, and neither needs a parser of our own.
    fn sample() -> Metadata {
        serde_json::from_str(
            r#"{
              "workspace_root": "/w",
              "workspace_members": ["core-a 0.1.0 (path+file:///w/core-a)",
                                    "adapter-x 0.1.0 (path+file:///w/adapter-x)"],
              "packages": [
                {
                  "id": "core-a 0.1.0 (path+file:///w/core-a)",
                  "name": "core-a",
                  "manifest_path": "/w/core-a/Cargo.toml",
                  "metadata": { "rha": { "role": "core", "implements": [] } },
                  "targets": [{ "kind": ["lib"] }, { "kind": ["custom-build"] }],
                  "dependencies": [
                    { "name": "adapter-x", "kind": null, "optional": true,
                      "target": "cfg(unix)", "rename": "ax" },
                    { "name": "tokio", "kind": "dev" }
                  ]
                },
                {
                  "id": "adapter-x 0.1.0 (path+file:///w/adapter-x)",
                  "name": "adapter-x",
                  "manifest_path": "/w/adapter-x/Cargo.toml",
                  "metadata": { "rha": { "implements": ["core-a::Port"] } },
                  "targets": [{ "kind": ["lib"] }],
                  "dependencies": []
                },
                {
                  "id": "stranger 0.1.0 (registry+x)",
                  "name": "stranger",
                  "manifest_path": "/elsewhere/Cargo.toml",
                  "targets": [],
                  "dependencies": []
                }
              ]
            }"#,
        )
        .expect("the sample parses")
    }

    #[test]
    fn only_workspace_members_become_crates() {
        let graph = build(&sample());
        let names: Vec<&str> = graph.crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["core-a", "adapter-x"]);
    }

    #[test]
    fn a_renamed_optional_target_dependency_keeps_every_field() {
        let graph = build(&sample());
        let edge = graph
            .edges_from("core-a")
            .find(|e| e.to.name() == "adapter-x")
            .expect("the edge is present under the PACKAGE name, not the rename");
        assert_eq!(edge.kind, DepKind::Normal);
        assert!(edge.optional);
        assert_eq!(edge.target_cfg.as_deref(), Some("cfg(unix)"));
        assert_eq!(edge.rename.as_deref(), Some("ax"));
        assert!(edge.to.is_member());
    }

    #[test]
    fn a_dependency_outside_the_workspace_is_external() {
        let graph = build(&sample());
        let edge = graph
            .edges_from("core-a")
            .find(|e| e.to.name() == "tokio")
            .expect("the dev-dependency is an edge too");
        assert_eq!(edge.kind, DepKind::Dev);
        assert!(!edge.to.is_member());
    }

    #[test]
    fn declared_metadata_is_carried_without_being_interpreted() {
        let graph = build(&sample());
        let core = graph.crate_named("core-a").expect("core-a is a member");
        assert_eq!(core.declared_role.as_deref(), Some("core"));
        assert!(
            core.has_build_script,
            "custom-build target means a build.rs"
        );
        // The graph does not classify: that is graph::classify's job.
        assert_eq!(core.role, None);

        let adapter = graph
            .crate_named("adapter-x")
            .expect("adapter-x is a member");
        assert_eq!(adapter.declared_role, None);
        assert_eq!(adapter.implements.len(), 1);
        assert_eq!(adapter.implements[0].owner, "core-a");
        assert!(!adapter.has_build_script);
    }

    #[test]
    fn a_missing_or_malformed_rha_table_is_absent_not_a_default_role() {
        assert_eq!(rha_metadata(None).role, None);
        let not_a_table = serde_json::json!({ "rha": "core" });
        assert_eq!(rha_metadata(Some(&not_a_table)).role, None);
        let empty = serde_json::json!({});
        assert!(rha_metadata(Some(&empty)).implements.is_empty());
    }
}
