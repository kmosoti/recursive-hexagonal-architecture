//! `rha-crates.toml`: the rules a crate graph is checked against (plan §5).
//!
//! A protected surface. The allow-lists decide what a core crate may depend
//! on, so widening one is a decision, not a fix for a failing check (§11.1).

use std::path::Path;

use serde::Deserialize;

use crate::error::{Context as _, Result};
use crate::graph::model::Role;
use crate::util::sha256_hex;

/// Where the rules live, relative to the workspace root.
pub const RULES_PATH: &str = "rha-crates.toml";

/// How a crate's role is decided when metadata does not say.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classification {
    #[serde(default)]
    pub adapter_prefix: String,
    #[serde(default)]
    pub app_prefix: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub harness: Vec<String>,
}

/// What a core crate may depend on.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Core {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub dev_allow: Vec<String>,
    #[serde(default)]
    pub allow_build_scripts: bool,
}

/// How strictly adapters are held to their declared ports.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adapters {
    #[serde(default = "yes")]
    pub require_port_owner_dependency: bool,
    /// `"warn"` or `"error"`: an adapter depending on a core whose port it
    /// does not implement.
    #[serde(default = "warn")]
    pub foreign_core_dependency: String,
}

const fn yes() -> bool {
    true
}

fn warn() -> String {
    "warn".to_owned()
}

impl Default for Adapters {
    fn default() -> Self {
        Self {
            require_port_owner_dependency: true,
            foreign_core_dependency: warn(),
        }
    }
}

/// An edge that must not exist.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forbidden {
    /// A package name, a glob, or the selector `role:<role>`.
    pub from: String,
    /// A package name or a glob.
    pub to: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Whether full dependency resolution is enabled (the §4.1 transitive hole).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transitive {
    #[serde(default)]
    pub enabled: bool,
}

/// The rules file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema_version: u32,
    pub classification: Classification,
    #[serde(default)]
    pub core: Core,
    #[serde(default)]
    pub adapters: Adapters,
    #[serde(default)]
    pub forbidden: Vec<Forbidden>,
    #[serde(default)]
    pub transitive: Transitive,
}

impl Rules {
    /// Parses a rules file.
    ///
    /// # Errors
    /// Returns the TOML error. Unknown fields are rejected, so a misspelled
    /// key is a usage error rather than a rule that silently does nothing.
    pub fn parse(text: &str) -> std::result::Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Reads a rules file, returning it with the digest of its bytes. The
    /// digest goes in the report: a finding is only as meaningful as the
    /// rules it was produced under.
    ///
    /// # Errors
    /// Fails if the file cannot be read or does not parse.
    pub fn load(path: &Path) -> Result<(Self, String)> {
        let bytes = std::fs::read(path).context(|| format!("reading {}", path.display()))?;
        let text = String::from_utf8(bytes.clone())
            .context(|| format!("{} is not UTF-8", path.display()))?;
        let rules = Self::parse(&text).context(|| format!("parsing {}", path.display()))?;
        Ok((rules, sha256_hex(&bytes)))
    }

    /// Whether `foreign_core_dependency` is configured as an error.
    #[must_use]
    pub fn foreign_core_is_error(&self) -> bool {
        self.adapters.foreign_core_dependency == "error"
    }
}

/// Matches a name against a pattern that may end in `*`.
///
/// Deliberately small: plan §5's examples are exact names and one trailing
/// wildcard (`aws-sdk-*`). A full glob engine would be a dependency and a
/// second syntax to learn, and nothing in the corpus needs one.
#[must_use]
pub fn glob_matches(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => pattern == name,
    }
}

/// Whether a forbidden rule's `from` selects a crate with this name and role.
///
/// `role:<role>` selects every crate of that role, which is how a rule can
/// say "no core may depend on tokio" without listing the cores (corpus C09).
#[must_use]
pub fn selector_matches(selector: &str, name: &str, role: Option<Role>) -> bool {
    match selector.strip_prefix("role:") {
        Some(wanted) => role.is_some_and(|r| r.as_str() == wanted),
        None => glob_matches(selector, name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repositorys_own_rules_parse() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent")
            .join(RULES_PATH);
        let (rules, digest) = Rules::load(&path).expect("rha-crates.toml parses");
        assert_eq!(rules.schema_version, 1);
        assert_eq!(rules.classification.tools, vec!["xtask".to_owned()]);
        assert_eq!(digest.len(), 64);
        // Empty by decision, not by accident (CHG-003): no core crate exists,
        // and none may depend on anything external.
        assert!(rules.core.allow.is_empty());
        assert!(rules.core.dev_allow.is_empty());
        assert!(!rules.core.allow_build_scripts);
        assert!(rules.forbidden.is_empty());
        assert!(!rules.transitive.enabled);
    }

    #[test]
    fn an_unknown_key_is_an_error_not_a_rule_that_does_nothing() {
        let text = "schema_version = 1\n[classification]\nadaptor_prefix = \"adapter-\"\n";
        assert!(Rules::parse(text).is_err());
    }

    #[test]
    fn a_trailing_wildcard_matches_a_prefix_and_an_exact_name_matches_itself() {
        assert!(glob_matches("aws-sdk-*", "aws-sdk-s3"));
        assert!(glob_matches("aws-sdk-*", "aws-sdk-"));
        assert!(!glob_matches("aws-sdk-*", "aws-sd"));
        assert!(glob_matches("tokio", "tokio"));
        assert!(!glob_matches("tokio", "tokio-util"));
        // A bare `*` matches everything, which is a rule someone may write.
        assert!(glob_matches("*", "anything"));
    }

    #[test]
    fn a_role_selector_matches_by_role_and_a_name_selector_by_name() {
        assert!(selector_matches("role:core", "core-b", Some(Role::Core)));
        assert!(!selector_matches(
            "role:core",
            "core-b",
            Some(Role::Adapter)
        ));
        // An unclassified crate matches no role selector; class.unclassified
        // reports it separately.
        assert!(!selector_matches("role:core", "mystery", None));
        assert!(selector_matches("core-a", "core-a", Some(Role::Core)));
        assert!(!selector_matches("core-a", "core-b", Some(Role::Core)));
    }

    #[test]
    fn the_defaults_are_the_strict_ones() {
        let rules = Rules::parse("schema_version = 1\n[classification]\n").expect("parses");
        assert!(rules.adapters.require_port_owner_dependency);
        assert_eq!(rules.adapters.foreign_core_dependency, "warn");
        assert!(!rules.core.allow_build_scripts);
        assert!(!rules.transitive.enabled);
        assert!(!rules.foreign_core_is_error());
    }
}
