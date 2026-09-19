//! Rule `effect.core_clippy_template` (CHG-001): every crate with role `core`
//! has a `clippy.toml` byte-equal to `xtask/templates/core-clippy.toml`, and
//! forbids the three lints that file configures.
//!
//! Clippy 0.1.98 uses the nearest `clippy.toml` and does not merge it with the
//! root file, and a `.clippy.toml` in the same directory wins over
//! `clippy.toml` with only a warning (ADR-0002). A core crate without its own
//! copy therefore gets no deny list, an edited copy can silently drop entries,
//! and a `.clippy.toml` beside the copy silently replaces it.
//!
//! The configuration decides which paths are reported; the lint level decides
//! whether a report stops the build, and an `#[allow]` in the crate sets that
//! level locally (ADR-0002 rule 5). The workspace lint table cannot forbid
//! these lints, because a macro that expands to a group allow is then `E0453`
//! and `clap`'s derives do exactly that (rule 8), so each core crate forbids
//! them at its own crate root and this rule checks the line.
//!
//! `cargo xtask architecture` runs this rule from CHG-003; until then only
//! tests call it.

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
    /// The crate root does not forbid every lint the template configures, so
    /// an `#[allow]` anywhere in the crate switches that part of the deny
    /// list off.
    Unforbidden,
}

/// The lints a core crate's root must forbid. The template configures these
/// three, and nothing else sets a level that an attribute cannot lower.
pub const FORBIDDEN_LINTS: [&str; 3] = [
    "clippy::disallowed_methods",
    "clippy::disallowed_types",
    "clippy::disallowed_macros",
];

/// The lints of [`FORBIDDEN_LINTS`] that `source` does not forbid at its
/// crate root. Inner attributes only: an `#[allow]` further down cannot
/// lower a lint the root forbids, which is the property being checked.
#[must_use]
pub fn unforbidden(source: &str) -> Vec<&'static str> {
    let mut forbidden = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("#![forbid(") {
        rest = &rest[start + "#![forbid(".len()..];
        let Some(end) = rest.find(")]") else { break };
        forbidden.push_str(&rest[..end]);
        forbidden.push(' ');
        rest = &rest[end..];
    }
    let forbidden: String = forbidden.split_whitespace().collect();
    FORBIDDEN_LINTS
        .into_iter()
        .filter(|lint| !forbidden.contains(lint))
        .collect()
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
    /// What is missing, where a digest does not say it: the lints a crate
    /// root leaves unforbidden.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
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
                detail: None,
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
        drop(finding);

        let root = core.dir.join("src/lib.rs");
        let source = std::fs::read_to_string(&root).unwrap_or_default();
        let missing = unforbidden(&source);
        if !missing.is_empty() {
            findings.push(Finding {
                rule: RULE_ID,
                krate: core.name.clone(),
                problem: Problem::Unforbidden,
                path: root.display().to_string(),
                expected_sha256: expected.clone(),
                actual_sha256: None,
                detail: Some(format!(
                    "not forbidden at the crate root: {}",
                    missing.join(", ")
                )),
            });
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

    const FORBIDDING_ROOT: &[u8] = b"#![forbid(\n    clippy::disallowed_methods,\n    clippy::disallowed_types,\n    clippy::disallowed_macros\n)]\n";

    /// A fresh crate directory holding `files`, unique per test and process,
    /// whose crate root forbids the lints unless a test overwrites it.
    fn crate_dir(test: &str, files: &[(&str, &[u8])]) -> CoreCrate {
        let dir = std::env::temp_dir().join(format!("rha-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), FORBIDDING_ROOT).unwrap();
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
    fn a_crate_root_that_does_not_forbid_the_lints_is_a_finding() {
        let core = crate_dir("unforbidden", &[("clippy.toml", &template())]);
        std::fs::write(core.dir.join("src/lib.rs"), b"pub fn f() {}\n").unwrap();
        let found = check_and_clean(&core);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].problem, Problem::Unforbidden);
        assert!(found[0].path.ends_with("lib.rs"));
        for lint in FORBIDDEN_LINTS {
            assert!(found[0].detail.as_ref().is_some_and(|d| d.contains(lint)));
        }
    }

    #[test]
    fn forbidding_only_some_of_the_lints_names_the_rest() {
        assert_eq!(
            unforbidden(std::str::from_utf8(FORBIDDING_ROOT).unwrap()),
            Vec::<&str>::new()
        );
        let partial = "#![forbid(clippy::disallowed_methods)]\npub fn f() {}\n";
        assert_eq!(
            unforbidden(partial),
            vec!["clippy::disallowed_types", "clippy::disallowed_macros"]
        );
        let deny_not_forbid = "#![deny(clippy::disallowed_methods)]\n";
        assert_eq!(unforbidden(deny_not_forbid).len(), 3);
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
