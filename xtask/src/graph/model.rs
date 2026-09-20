//! The crate graph a rule is evaluated against (plan §5).
//!
//! Nothing here reads a file. The graph is built from `cargo metadata` output
//! in [`crate::metadata`] and from nothing else: plan §8 W3 requires that
//! renamed, optional, target-conditional and workspace-inherited dependencies
//! be resolved through metadata fields, **never** by parsing `Cargo.toml`
//! text. Corpus cases C15 to C18 are four spellings of one edge for exactly
//! this reason, and a checker that reads the manifest text misses one of them.

use std::path::PathBuf;

use serde::Serialize;

/// What a crate is, in the Rust profile (§6.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Core,
    Adapter,
    App,
    Tool,
    Harness,
}

impl Role {
    /// The spelling used in `[package.metadata.rha] role` and in reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Adapter => "adapter",
            Self::App => "app",
            Self::Tool => "tool",
            Self::Harness => "harness",
        }
    }

    /// Parses a declared role, or `None` when the value is not one of the five.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "core" => Some(Self::Core),
            "adapter" => Some(Self::Adapter),
            "app" => Some(Self::App),
            "tool" => Some(Self::Tool),
            "harness" => Some(Self::Harness),
            _ => None,
        }
    }

    /// Whether a crate of this role may depend on an adapter (D5): only a
    /// composition root and a test harness may.
    #[must_use]
    pub const fn may_depend_on_adapter(self) -> bool {
        matches!(self, Self::App | Self::Harness)
    }
}

/// How a crate's role was decided. Plan §5 fixes the precedence, and it is
/// recorded per crate so a report can be argued with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleSource {
    /// `[package.metadata.rha] role`. Wins over everything.
    Metadata,
    /// The `adapter-` or `app-` prefix of `[classification]`.
    Prefix,
    /// The `tools` or `harness` list of `[classification]`.
    List,
}

/// A port an adapter claims to implement, written `owner::Port`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortRef {
    /// The whole string as declared.
    pub path: String,
    /// The part before the first `::`: the crate expected to own the port.
    pub owner: String,
}

impl PortRef {
    /// Splits `owner::Port`. A string with no `::` names an owner and no port,
    /// which `meta.unknown_port_owner` will report unless a member matches.
    #[must_use]
    pub fn parse(path: &str) -> Self {
        let owner = path.split("::").next().unwrap_or(path).to_owned();
        Self {
            path: path.to_owned(),
            owner,
        }
    }
}

/// A member of the workspace under check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrateNode {
    pub name: String,
    pub manifest_path: PathBuf,
    /// `None` when nothing classified it, which is `class.unclassified`.
    pub role: Option<Role>,
    pub role_source: Option<RoleSource>,
    /// A role declared in metadata that disagrees with the prefix, kept so
    /// `class.prefix_role_conflict` can name both sides.
    pub declared_role: Option<String>,
    pub implements: Vec<PortRef>,
    pub has_build_script: bool,
}

/// The kind of a dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

impl DepKind {
    /// `cargo metadata` writes `null` for a normal dependency.
    #[must_use]
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("dev") => Self::Dev,
            Some("build") => Self::Build,
            _ => Self::Normal,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Dev => "dev",
            Self::Build => "build",
        }
    }
}

/// Where an edge points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    /// Another member of this workspace.
    Member { name: String },
    /// A crate outside the workspace: a registry crate, or a path dependency
    /// that is not a member.
    External { name: String },
}

impl Target {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Member { name } | Self::External { name } => name,
        }
    }

    #[must_use]
    pub const fn is_member(&self) -> bool {
        matches!(self, Self::Member { .. })
    }
}

/// One declared dependency.
///
/// `to` carries the **package** name. A renamed dependency is declared as
/// `ax = { package = "adapter-x" }`, and `cargo metadata` reports `name` as
/// the package and `rename` separately; classifying by the key would miss
/// corpus case C17.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: Target,
    pub kind: DepKind,
    /// `true` for `optional = true`. An optional dependency is still a
    /// declared edge (§6.12, corpus C16).
    pub optional: bool,
    /// The `cfg(...)` of a `[target.'cfg(...)'.dependencies]` entry (C15).
    pub target_cfg: Option<String>,
    /// The key used in `Cargo.toml` when it differs from the package name (C17).
    pub rename: Option<String>,
}

impl Edge {
    /// Where the dependency is declared, for a witness a reader can check.
    #[must_use]
    pub fn declared_in(&self) -> String {
        let section = match self.kind {
            DepKind::Normal => "dependencies",
            DepKind::Dev => "dev-dependencies",
            DepKind::Build => "build-dependencies",
        };
        match &self.target_cfg {
            Some(cfg) => format!("[target.'{cfg}'.{section}]"),
            None => format!("[{section}]"),
        }
    }
}

/// How the metadata was gathered. Recorded in the report, because a rule that
/// needs resolved dependencies cannot be evaluated in `NoDeps` mode and says
/// so rather than passing quietly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataMode {
    /// `cargo metadata --no-deps`: direct declared edges only.
    NoDeps,
    /// Full resolution, for `[transitive] enabled = true`.
    Resolved,
}

/// The workspace under check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrateGraph {
    pub workspace_root: PathBuf,
    pub crates: Vec<CrateNode>,
    pub edges: Vec<Edge>,
    pub mode: MetadataMode,
}

impl CrateGraph {
    #[must_use]
    pub fn crate_named(&self, name: &str) -> Option<&CrateNode> {
        self.crates.iter().find(|c| c.name == name)
    }

    #[must_use]
    pub fn role_of(&self, name: &str) -> Option<Role> {
        self.crate_named(name).and_then(|c| c.role)
    }

    /// Edges out of `name`, in declaration order.
    pub fn edges_from<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Edge> {
        self.edges.iter().filter(move |e| e.from == name)
    }

    /// How many edges of each kind were examined, for the report.
    #[must_use]
    pub fn edge_counts(&self) -> (usize, usize, usize) {
        let mut counts = (0, 0, 0);
        for edge in &self.edges {
            match edge.kind {
                DepKind::Normal => counts.0 += 1,
                DepKind::Dev => counts.1 += 1,
                DepKind::Build => counts.2 += 1,
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_port_reference_names_its_owner() {
        let port = PortRef::parse("core-a::Port");
        assert_eq!(
            (port.owner.as_str(), port.path.as_str()),
            ("core-a", "core-a::Port")
        );
        // No separator: the whole string is the owner, and no member will
        // match it, so meta.unknown_port_owner reports it.
        assert_eq!(PortRef::parse("ghost").owner, "ghost");
    }

    #[test]
    fn a_normal_dependency_is_the_null_kind() {
        assert_eq!(DepKind::parse(None), DepKind::Normal);
        assert_eq!(DepKind::parse(Some("dev")), DepKind::Dev);
        assert_eq!(DepKind::parse(Some("build")), DepKind::Build);
    }

    #[test]
    fn only_roots_and_harnesses_may_depend_on_an_adapter() {
        assert!(Role::App.may_depend_on_adapter());
        assert!(Role::Harness.may_depend_on_adapter());
        for role in [Role::Core, Role::Adapter, Role::Tool] {
            assert!(!role.may_depend_on_adapter(), "{role:?}");
        }
    }

    #[test]
    fn a_witness_names_the_section_the_edge_is_declared_in() {
        let edge = |kind, cfg: Option<&str>| Edge {
            from: "core-a".to_owned(),
            to: Target::Member {
                name: "adapter-x".to_owned(),
            },
            kind,
            optional: false,
            target_cfg: cfg.map(ToOwned::to_owned),
            rename: None,
        };
        assert_eq!(edge(DepKind::Normal, None).declared_in(), "[dependencies]");
        assert_eq!(edge(DepKind::Dev, None).declared_in(), "[dev-dependencies]");
        assert_eq!(
            edge(DepKind::Normal, Some("cfg(unix)")).declared_in(),
            "[target.'cfg(unix)'.dependencies]"
        );
    }
}
