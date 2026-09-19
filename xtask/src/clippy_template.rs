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

    fn template() -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/core-clippy.toml"))
            .unwrap()
    }

    /// A fresh crate directory holding `files`, unique per test and process.
    fn crate_dir(test: &str, files: &[(&str, &[u8])]) -> CoreCrate {
        let dir = std::env::temp_dir().join(format!("rha-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, bytes) in files {
            std::fs::write(dir.join(name), bytes).unwrap();
        }
        CoreCrate {
            name: test.to_owned(),
            dir,
        }
    }

    fn check_and_clean(core: &CoreCrate) -> Vec<Finding> {
        let found = check(&template(), std::slice::from_ref(core));
        std::fs::remove_dir_all(&core.dir).unwrap();
        found
    }

    #[test]
    fn a_copy_of_the_template_conforms() {
        let core = crate_dir("conforms", &[("clippy.toml", &template())]);
        assert_eq!(check_and_clean(&core), vec![]);
    }

    #[test]
    fn a_copy_that_differs_is_a_finding_with_both_digests() {
        let edited = b"disallowed-methods = []\n";
        let core = crate_dir("differs", &[("clippy.toml", edited)]);
        let found = check_and_clean(&core);
        assert_eq!(found.len(), 1);
        let finding = &found[0];
        assert_eq!((finding.rule, finding.problem), (RULE_ID, Problem::Differs));
        assert_eq!(
            finding.actual_sha256.as_deref(),
            Some(sha256_hex(edited).as_str())
        );
        assert_eq!(finding.expected_sha256, sha256_hex(&template()));
    }

    #[test]
    fn a_missing_file_is_a_finding() {
        let core = crate_dir("missing", &[]);
        let found = check_and_clean(&core);
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].problem, found[0].actual_sha256.as_deref()),
            (Problem::Missing, None)
        );
    }

    #[test]
    fn a_dotfile_beside_a_correct_copy_is_a_finding() {
        let core = crate_dir(
            "shadowed",
            &[
                ("clippy.toml", &template()),
                (".clippy.toml", b"allow-unwrap-in-tests = true\n"),
            ],
        );
        let found = check_and_clean(&core);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].problem, Problem::Shadowed);
        assert!(found[0].path.ends_with(".clippy.toml"));
    }
}
