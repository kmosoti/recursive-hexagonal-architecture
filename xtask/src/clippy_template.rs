//! Rule `effect.core_clippy_template` (CHG-001): every crate with role `core`
//! has a `clippy.toml` byte-equal to `xtask/templates/core-clippy.toml`.
//!
//! Clippy 0.1.98 uses the nearest `clippy.toml` and does not merge it with the
//! root file, and a `.clippy.toml` in the same directory wins over
//! `clippy.toml` with only a warning (ADR-0002). A core crate without its own
//! copy therefore gets no deny list, an edited copy can silently drop entries,
//! and a `.clippy.toml` beside the copy silently replaces it. `cargo xtask
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

/// What is wrong with a core crate's Clippy configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Problem {
    /// `clippy.toml` is missing or unreadable.
    Missing,
    /// `clippy.toml` differs from the template.
    Differs,
    /// A `.clippy.toml` in the crate directory takes precedence over `clippy.toml`.
    Shadowed,
}

/// A violation. The witness is the path and the digests: anyone can
/// recompute them with `sha256sum` and compare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule: &'static str,
    #[serde(rename = "crate")]
    pub krate: String,
    pub problem: Problem,
    pub path: String,
    pub expected_sha256: String,
    /// The digest of the file at `path`; `None` when it is missing or unreadable.
    pub actual_sha256: Option<String>,
}

/// Checks each core crate's Clippy configuration against `template`.
#[must_use]
pub fn check(template: &[u8], cores: &[CoreCrate]) -> Vec<Finding> {
    let expected = sha256_hex(template);
    let mut findings = Vec::new();
    for core in cores {
        let mut finding = |problem, path: PathBuf, actual: Option<Vec<u8>>| {
            findings.push(Finding {
                rule: RULE_ID,
                krate: core.name.clone(),
                problem,
                path: path.display().to_string(),
                expected_sha256: expected.clone(),
                actual_sha256: actual.map(|bytes| sha256_hex(&bytes)),
            });
        };
        let path = core.dir.join("clippy.toml");
        match std::fs::read(&path) {
            Ok(bytes) if bytes == template => {}
            Ok(bytes) => finding(Problem::Differs, path, Some(bytes)),
            Err(_) => finding(Problem::Missing, path, None),
        }
        let dotfile = core.dir.join(".clippy.toml");
        if dotfile.exists() {
            let bytes = std::fs::read(&dotfile).ok();
            finding(Problem::Shadowed, dotfile, bytes);
        }
    }
    findings
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
            core("core-b", "core-seeded/crates/core-b"),
            core("core-every", "core-seeded/crates/core-every"),
        ];
        assert_eq!(check(&template(), &cores), vec![]);
    }

    #[test]
    fn a_deny_list_without_the_test_allowances_is_a_finding() {
        let found = check(&template(), &[core("core-a", "discovery/crates/core-a")]);
        assert_eq!(found.len(), 1);
        let finding = &found[0];
        assert_eq!((finding.rule, finding.problem), (RULE_ID, Problem::Differs));
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
        assert_eq!(
            (found[0].problem, found[0].actual_sha256.as_deref()),
            (Problem::Missing, None)
        );
    }

    #[test]
    fn a_dotfile_beside_a_correct_copy_is_a_finding() {
        let dir = std::env::temp_dir().join(format!("rha-clippy-shadow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("clippy.toml"), template()).unwrap();
        std::fs::write(dir.join(".clippy.toml"), b"allow-unwrap-in-tests = true\n").unwrap();
        let found = check(
            &template(),
            &[CoreCrate {
                name: "core-s".into(),
                dir: dir.clone(),
            }],
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].problem, Problem::Shadowed);
        assert!(found[0].path.ends_with(".clippy.toml"));
    }
}
