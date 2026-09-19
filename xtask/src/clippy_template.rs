//! Rule `effect.core_clippy_template` (CHG-001): every crate with role `core`
//! has a `clippy.toml` byte-equal to `xtask/templates/core-clippy.toml`.
//!
//! Clippy 0.1.98 uses the nearest `clippy.toml` and does not merge it with the
//! root file (ADR-0002). A core crate without its own copy therefore gets no
//! deny list, and an edited copy can silently drop entries. `cargo xtask
//! architecture` runs this rule from CHG-003; until then only tests call it.

use std::path::PathBuf;

use serde::Serialize;

use crate::util::sha256_hex;

pub const RULE_ID: &str = "effect.core_clippy_template";

/// Where the template lives, relative to the workspace root.
pub const TEMPLATE_PATH: &str = "xtask/templates/core-clippy.toml";

/// A crate the rule applies to.
#[derive(Debug, Clone)]
pub struct CoreCrate {
    pub name: String,
    /// The directory holding the crate's `Cargo.toml`.
    pub dir: PathBuf,
}

/// A violation. The witness is the path and the two digests: anyone can
/// recompute both with `sha256sum` and compare them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule: &'static str,
    #[serde(rename = "crate")]
    pub krate: String,
    pub path: String,
    pub expected_sha256: String,
    /// `None` when the file is missing or unreadable.
    pub actual_sha256: Option<String>,
}

/// Checks each core crate's `clippy.toml` against `template`.
#[must_use]
pub fn check(template: &[u8], cores: &[CoreCrate]) -> Vec<Finding> {
    let expected = sha256_hex(template);
    cores
        .iter()
        .filter_map(|core| {
            let path = core.dir.join("clippy.toml");
            match std::fs::read(&path) {
                Ok(bytes) if bytes == template => None,
                other => Some(Finding {
                    rule: RULE_ID,
                    krate: core.name.clone(),
                    path: path.display().to_string(),
                    expected_sha256: expected.clone(),
                    actual_sha256: other.ok().map(|bytes| sha256_hex(&bytes)),
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn corpus(rel: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/corpus/clippy")
            .join(rel)
    }

    fn template() -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/core-clippy.toml"))
            .unwrap()
    }

    fn core(name: &str, rel: &str) -> CoreCrate {
        CoreCrate {
            name: name.to_owned(),
            dir: corpus(rel),
        }
    }

    #[test]
    fn copies_of_the_template_conform() {
        let cores = [
            core("core-a", "core-seeded/crates/core-a"),
            core("core-every", "core-seeded/crates/core-every"),
        ];
        assert_eq!(check(&template(), &cores), vec![]);
    }

    #[test]
    fn a_deny_list_without_the_test_allowances_is_a_finding() {
        let found = check(&template(), &[core("core-a", "discovery/crates/core-a")]);
        assert_eq!(found.len(), 1);
        let finding = &found[0];
        assert_eq!(finding.rule, RULE_ID);
        let actual = std::fs::read(corpus("discovery/crates/core-a/clippy.toml")).unwrap();
        assert_eq!(
            finding.actual_sha256.as_deref(),
            Some(sha256_hex(&actual).as_str())
        );
        assert_eq!(finding.expected_sha256, sha256_hex(&template()));
    }

    #[test]
    fn a_missing_file_is_a_finding() {
        let found = check(
            &template(),
            &[core("adapter-x", "discovery/crates/adapter-x")],
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].actual_sha256, None);
    }
}
