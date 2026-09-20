//! Typed model of `.rha/policy.toml`, its digest, and the §12.1 drift check.
//!
//! The policy file is the single owner of check ids and parameters (spec
//! §11.7.1). This module only reads it; nothing here decides outcomes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Context as _, Error, Result};
use crate::util::sha256_hex;

pub const POLICY_PATH: &str = ".rha/policy.toml";

/// The heading line that opens the L0 command block in the spec (§12.1).
pub const SPEC_L0_MARKER: &str = "L0 commands (RHA-Rust)";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: u32,
    pub authority: Authority,
    pub repair_loop: RepairLoop,
    pub proptest: BTreeMap<String, u32>,
    pub evidence: EvidencePolicy,
    pub surface: Surface,
    pub strictness: BTreeMap<String, String>,
    pub lanes: BTreeMap<String, Lane>,
    /// Acceptance by merge and the decision ledger (CHG-002.2). Optional so a
    /// base revision written before that change still parses.
    #[serde(default)]
    pub acceptance: Option<Acceptance>,
}

/// What counts as the acceptance event, who materializes the record, and
/// where decision-point status lives (`[acceptance]`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    pub by_merge: bool,
    pub record_written_by: String,
    pub record_deadline: String,
    pub record_creation_preapproved: bool,
    pub decisions_ledger: String,
    pub defaults_apply_when_unanswered: bool,
    pub applies_to_merges_after: String,
    pub bootstrap: Bootstrap,
    pub ledger_edits: LedgerEdits,
}

/// The bounded period in which some acceptance predicates cannot hold
/// (`[acceptance.bootstrap]`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub authentic_unsatisfiable_until: String,
    pub passed_unsatisfiable_until: String,
    pub every_acceptance_in_this_period: String,
    pub expires: String,
}

/// Which edits to the protected decision ledger are pre-approved
/// (`[acceptance.ledger_edits]`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEdits {
    pub preapproved: Vec<String>,
    pub everything_else: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub profile: String,
    pub acceptance_authority: Vec<String>,
    pub exception_authority: Vec<String>,
    pub cooling_off_hours: u32,
    pub exception_log: String,
    pub trusted_producers: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairLoop {
    pub max_attempts: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePolicy {
    pub digest_algorithm: String,
    pub required_inputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    pub default_lanes: Vec<String>,
    pub protected: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lane {
    pub source: String,
    pub cadence: String,
    pub acceptance: bool,
    pub checks: Vec<Check>,
}

impl Lane {
    /// The spec path named in `source`, before the first space.
    #[must_use]
    pub fn source_path(&self) -> &str {
        self.source.split_whitespace().next().unwrap_or("")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    pub kind: String,
    pub argv: Vec<String>,
    pub tool: Option<String>,
    pub validity: Option<Validity>,
    /// A report file the command writes; deleted before the run so only a
    /// fresh report can satisfy validity.
    pub report: Option<String>,
    /// Key into `[proptest]`; its value is exported as `PROPTEST_CASES`.
    pub proptest: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_true")]
    pub waivable: bool,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub triggers: Vec<String>,
    pub note: Option<String>,
}

fn default_true() -> bool {
    true
}

/// The criterion that an exit status of zero must also meet before a check
/// counts as `passed` (`Valid_kind`, §11.7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validity {
    ExitZero,
    JunitTestsSelected,
    DoctestHarnessRan,
    ArchitectureReportPassed,
}

/// A policy together with where it was read from and its digest.
#[derive(Debug)]
pub struct LoadedPolicy {
    pub policy: Policy,
    pub path: PathBuf,
    /// `sha256:<64 hex>` over the file's bytes as read.
    pub digest: String,
}

/// Reads and parses `.rha/policy.toml` under `root`.
///
/// # Errors
/// Fails if the file cannot be read or does not match the schema.
pub fn load(root: &Path) -> Result<LoadedPolicy> {
    let path = root.join(POLICY_PATH);
    let bytes = std::fs::read(&path).context(|| format!("reading {}", path.display()))?;
    let text = std::str::from_utf8(&bytes).context(|| format!("decoding {}", path.display()))?;
    let policy: Policy = toml::from_str(text).context(|| format!("parsing {}", path.display()))?;
    Ok(LoadedPolicy {
        policy,
        path,
        digest: format!("sha256:{}", sha256_hex(&bytes)),
    })
}

/// Structural problems that make a policy unusable. Empty means usable.
#[must_use]
pub fn validate(policy: &Policy) -> Vec<String> {
    let mut problems = Vec::new();
    if policy.schema_version != 1 {
        problems.push(format!(
            "unsupported schema_version {}",
            policy.schema_version
        ));
    }
    let mut seen = BTreeSet::new();
    for (lane_name, lane) in &policy.lanes {
        for check in &lane.checks {
            if !seen.insert(check.id.as_str()) {
                problems.push(format!("duplicate check id {}", check.id));
            }
            if !check.id.starts_with(&format!("{lane_name}.")) {
                problems.push(format!(
                    "check {} is not prefixed by its lane {lane_name}",
                    check.id
                ));
            }
            if let Some(key) = &check.proptest
                && !policy.proptest.contains_key(key)
            {
                problems.push(format!(
                    "check {} names unknown proptest budget {key}",
                    check.id
                ));
            }
            if !check.argv.is_empty() && check.validity.is_none() {
                problems.push(format!(
                    "check {} has a command but no validity criterion",
                    check.id
                ));
            }
            if check.required && check.argv.is_empty() {
                problems.push(format!("required check {} has no command", check.id));
            }
        }
    }
    for lane in &policy.surface.default_lanes {
        if !policy.lanes.contains_key(lane) {
            problems.push(format!("surface.default_lanes names unknown lane {lane}"));
        }
    }
    problems
}

/// The command lines of the fenced block that follows [`SPEC_L0_MARKER`],
/// with `#` comments removed, each split on whitespace.
///
/// # Errors
/// Fails if the marker or the closing fence is missing, or the block is empty.
pub fn spec_l0_commands(spec_text: &str) -> Result<Vec<Vec<String>>> {
    let mut lines = spec_text.lines();
    lines
        .by_ref()
        .find(|line| line.trim() == SPEC_L0_MARKER)
        .ok_or_else(|| Error::new(format!("spec has no line `{SPEC_L0_MARKER}`")))?;
    let mut commands = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            if commands.is_empty() {
                return Err(Error::new("the L0 command block is empty"));
            }
            return Ok(commands);
        }
        let code = line.split('#').next().unwrap_or("");
        let argv: Vec<String> = code.split_whitespace().map(str::to_owned).collect();
        if !argv.is_empty() {
            commands.push(argv);
        }
    }
    Err(Error::new("the L0 command block has no closing fence"))
}

/// Differences between the policy's argv lists and the spec's, by position.
#[must_use]
pub fn drift(policy_argv: &[Vec<String>], spec_argv: &[Vec<String>]) -> Vec<String> {
    let mut out = Vec::new();
    if policy_argv.len() != spec_argv.len() {
        out.push(format!(
            "policy has {} commands, spec has {}",
            policy_argv.len(),
            spec_argv.len()
        ));
    }
    for (i, (ours, theirs)) in policy_argv.iter().zip(spec_argv).enumerate() {
        if ours != theirs {
            out.push(format!(
                "command {}: policy `{}` != spec `{}`",
                i + 1,
                ours.join(" "),
                theirs.join(" ")
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC_SNIPPET: &str = "intro\n\n```text\nL0 commands (RHA-Rust)\n  cargo fmt --all -- --check\n  cargo clippy --workspace   # a comment\n\n  typos\n```\nafter\n";

    fn argv(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn extracts_commands_and_strips_comments() {
        let commands = spec_l0_commands(SPEC_SNIPPET).unwrap();
        assert_eq!(
            commands,
            vec![
                argv("cargo fmt --all -- --check"),
                argv("cargo clippy --workspace"),
                argv("typos"),
            ]
        );
    }

    #[test]
    fn missing_marker_or_fence_is_an_error() {
        assert!(spec_l0_commands("no marker here").is_err());
        assert!(spec_l0_commands("L0 commands (RHA-Rust)\n  cargo fmt\n").is_err());
    }

    #[test]
    fn drift_reports_a_changed_argument_and_a_length_change() {
        let spec = vec![argv("cargo fmt --all -- --check"), argv("typos")];
        let mut ours = spec.clone();
        assert!(drift(&ours, &spec).is_empty());
        ours[0] = argv("cargo fmt --all");
        let found = drift(&ours, &spec);
        assert_eq!(found.len(), 1);
        assert!(found[0].starts_with("command 1:"));
        ours.push(argv("cargo machete"));
        assert!(drift(&ours, &spec).iter().any(|d| d.contains("3 commands")));
    }
}
