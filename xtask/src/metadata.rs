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
    /// The dependency's directory. Present only for a path dependency.
    #[serde(default)]
    path: Option<PathBuf>,
    /// `null` for a path dependency; a `registry+…` or `git+…` URL otherwise.
    #[serde(default)]
    source: Option<String>,
}

/// What `[package.metadata.rha]` may declare.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RhaMetadata {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    implements: Vec<String>,
    /// Reserved for the module-level rules file introduced by W5. It is
    /// validated here so a valid metadata declaration is not rejected before
    /// module checking exists.
    #[serde(default, rename = "composite")]
    _composite: Option<String>,
}

/// Why loading the graph failed. Configuration errors come from the checked
/// workspace itself; environment errors mean cargo could not provide usable
/// metadata at all.
#[derive(Debug)]
pub enum LoadError {
    Configuration(Error),
    Environment(Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(error) | Self::Environment(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LoadError {}

/// Runs `cargo metadata` and builds the graph.
///
/// # Errors
/// Cargo spawn, nonzero exit, and invalid metadata JSON are environment
/// failures, reported with exit code 3. A malformed `package.metadata.rha`
/// declaration is a configuration failure, reported with exit code 2.
pub fn load(
    root: &Path,
    manifest_path: Option<&Path>,
) -> std::result::Result<CrateGraph, LoadError> {
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
    let output = command.output().map_err(|error| {
        LoadError::Environment(Error::new(format!("running `cargo metadata`: {error}")))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LoadError::Environment(Error::new(format!(
            "`cargo metadata` exited {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ))));
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout).map_err(|error| {
        LoadError::Environment(Error::new(format!(
            "parsing `cargo metadata` output: {error}"
        )))
    })?;
    build(&metadata).map_err(LoadError::Configuration)
}

/// Builds the graph from parsed metadata. Split out so tests can drive it
/// with recorded output instead of running cargo. Malformed RHA declarations
/// are returned as configuration errors with crate and manifest context.
fn build(metadata: &Metadata) -> Result<CrateGraph> {
    let members: Vec<&Package> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .collect();
    // A member is identified by where it lives, not by what it is called.
    // `cargo metadata` reports a path dependency with `path` set and `source`
    // null; a registry or git package has a source and no path, and is
    // external however familiar its name. Matching by name alone let a core
    // reach a registry crate that shares a member's name without the edge
    // appearing in [core] allow, because the effect rules skip member targets
    // (review finding on pull request 6, CHG-003.1).
    let member_dirs: Vec<(&str, &Path)> = members
        .iter()
        .filter_map(|p| p.manifest_path.parent().map(|dir| (p.name.as_str(), dir)))
        .collect();

    let mut crates = Vec::with_capacity(members.len());
    let mut edges = Vec::new();
    for package in &members {
        let rha = rha_metadata(package)?;
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
            let member = dep
                .path
                .as_deref()
                .filter(|_| dep.source.is_none())
                .and_then(|dir| member_dirs.iter().find(|(_, d)| *d == dir));
            let to = match member {
                Some((name, _)) => Target::Member {
                    name: (*name).to_owned(),
                },
                None => Target::External {
                    name: dep.name.clone(),
                },
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
    Ok(CrateGraph {
        workspace_root: metadata.workspace_root.clone(),
        crates,
        edges,
        mode: MetadataMode::NoDeps,
    })
}

/// Reads `[package.metadata.rha]`, tolerating its absence. When the table is
/// present, its shape is configuration and malformed values must stop the run
/// with the crate and manifest named in the error.
fn rha_metadata(package: &Package) -> Result<RhaMetadata> {
    let Some(metadata) = package.metadata.as_ref() else {
        return Ok(RhaMetadata::default());
    };
    let Some(metadata) = metadata.as_object() else {
        return Err(Error::new(format!(
            "crate {} manifest {} has malformed package.metadata: expected a table",
            package.name,
            package.manifest_path.display()
        )));
    };
    let Some(rha) = metadata.get("rha") else {
        return Ok(RhaMetadata::default());
    };
    if !rha.is_object() {
        return Err(Error::new(format!(
            "crate {} manifest {} has malformed package.metadata.rha: expected a table",
            package.name,
            package.manifest_path.display()
        )));
    }
    if let Some(role) = rha.get("role")
        && !role.is_null()
        && !role.is_string()
    {
        return Err(Error::new(format!(
            "crate {} manifest {} has malformed package.metadata.rha.role: expected a string",
            package.name,
            package.manifest_path.display()
        )));
    }
    if let Some(implements) = rha.get("implements")
        && !implements.is_array()
    {
        return Err(Error::new(format!(
            "crate {} manifest {} has malformed package.metadata.rha.implements: expected an array",
            package.name,
            package.manifest_path.display()
        )));
    }
    serde_json::from_value(rha.clone()).context(|| {
        format!(
            "crate {} manifest {} has malformed package.metadata.rha",
            package.name,
            package.manifest_path.display()
        )
    })
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
                                    "adapter-x 0.1.0 (path+file:///w/adapter-x)",
                                    "core-b 0.1.0 (path+file:///w/core-b)"],
              "packages": [
                {
                  "id": "core-a 0.1.0 (path+file:///w/core-a)",
                  "name": "core-a",
                  "manifest_path": "/w/core-a/Cargo.toml",
                  "metadata": { "rha": { "role": "core", "implements": [] } },
                  "targets": [{ "kind": ["lib"] }, { "kind": ["custom-build"] }],
                  "dependencies": [
                    { "name": "adapter-x", "kind": null, "optional": true,
                      "target": "cfg(unix)", "rename": "ax",
                      "source": null, "path": "/w/adapter-x" },
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
                  "id": "core-b 0.1.0 (path+file:///w/core-b)",
                  "name": "core-b",
                  "manifest_path": "/w/core-b/Cargo.toml",
                  "metadata": { "rha": { "role": "core" } },
                  "targets": [{ "kind": ["lib"] }],
                  "dependencies": [
                    { "name": "adapter-x", "kind": null,
                      "source": "registry+https://github.com/rust-lang/crates.io-index" },
                    { "name": "core-a", "kind": null, "source": null,
                      "path": "/elsewhere/core-a" }
                  ]
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
        let graph = build(&sample()).expect("sample graph builds");
        let names: Vec<&str> = graph.crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["core-a", "adapter-x", "core-b"]);
    }

    #[test]
    fn a_package_is_a_member_by_where_it_lives_not_by_its_name() {
        // core-b names two packages that share a member's name. Neither lives
        // at the member's directory, so neither is the member: a registry
        // crate called adapter-x is not the workspace's adapter-x, and a path
        // crate called core-a outside the workspace is not its core-a. Both
        // are external, and therefore subject to [core] allow.
        let graph = build(&sample()).expect("sample graph builds");
        let targets: Vec<(&str, bool)> = graph
            .edges_from("core-b")
            .map(|e| (e.to.name(), e.to.is_member()))
            .collect();
        assert_eq!(targets, vec![("adapter-x", false), ("core-a", false)]);
    }

    #[test]
    fn a_renamed_optional_target_dependency_keeps_every_field() {
        let graph = build(&sample()).expect("sample graph builds");
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
        let graph = build(&sample()).expect("sample graph builds");
        let edge = graph
            .edges_from("core-a")
            .find(|e| e.to.name() == "tokio")
            .expect("the dev-dependency is an edge too");
        assert_eq!(edge.kind, DepKind::Dev);
        assert!(!edge.to.is_member());
    }

    #[test]
    fn declared_metadata_is_carried_without_being_interpreted() {
        let graph = build(&sample()).expect("sample graph builds");
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
    fn a_missing_rha_table_is_absent_not_a_default_role() {
        let mut package = sample().packages.remove(0);
        package.metadata = None;
        assert_eq!(rha_metadata(&package).expect("absent metadata").role, None);
        let empty = serde_json::json!({});
        package.metadata = Some(empty);
        assert!(
            rha_metadata(&package)
                .expect("absent rha table")
                .implements
                .is_empty()
        );
    }

    #[test]
    fn malformed_rha_table_is_a_configuration_error_with_context() {
        let mut package = sample().packages.remove(0);
        for (index, value) in [
            serde_json::json!(null),
            serde_json::json!("core"),
            serde_json::json!([]),
            serde_json::json!({ "implements": "core-a::Port" }),
            serde_json::json!({ "role": 7 }),
            serde_json::json!({ "unknown": true }),
        ]
        .into_iter()
        .enumerate()
        {
            package.metadata = Some(serde_json::json!({ "rha": value }));
            let error = match rha_metadata(&package) {
                Ok(got) => panic!("malformed metadata case {index} unexpectedly parsed: {got:?}"),
                Err(error) => error,
            };
            let text = error.to_string();
            assert!(text.contains("core-a"), "{text}");
            assert!(text.contains("Cargo.toml"), "{text}");
        }
    }

    #[test]
    fn the_declared_composite_rules_file_is_accepted_for_later_module_checks() {
        let mut package = sample().packages.remove(0);
        package.metadata = Some(serde_json::json!({
            "rha": { "role": "core", "composite": "rha-modules.toml" }
        }));
        assert_eq!(
            rha_metadata(&package)
                .expect("valid metadata")
                .role
                .as_deref(),
            Some("core")
        );
    }
}
