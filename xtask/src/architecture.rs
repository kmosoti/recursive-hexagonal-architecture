//! `cargo xtask architecture`: the crate-graph check (plan §5; spec §6.13).
//!
//! The crate level is implemented from CHG-003. The module level arrives in
//! CHG-007 and until then every crate's `module_checks` entry reports
//! `not_run` with a reason, never an empty findings list that would read like
//! a pass (§11.4).
//!
//! Exit codes are fixed by plan §5 and are the contract the lane relies on:
//!
//! | code | meaning |
//! | --- | --- |
//! | 0 | no error findings |
//! | 1 | error findings |
//! | 2 | usage or configuration error |
//! | 3 | environment failure (`cargo metadata` failed) — **never a pass** |
//!
//! Three matters because it is the one an exit-status reader is most likely
//! to get wrong: a checker that cannot run is not a checker that found
//! nothing.

use std::path::{Path, PathBuf};

use crate::error::{Context as _, Result};
use crate::graph::check;
use crate::graph::report;
use crate::graph::rules::{RULES_PATH, Rules};
use crate::lanes::RUN_DIR;
use crate::{clippy_template, metadata};

/// Exit status meaning "the check did not run; see the report".
pub const EXIT_NOT_RUN: i32 = 4;
/// Exit status for a usage or configuration error.
pub const EXIT_USAGE: i32 = 2;
/// Exit status for an environment failure. Never a pass.
pub const EXIT_ENVIRONMENT: i32 = 3;

/// Where the architecture report is written.
pub const REPORT_PATH: &str = "target/rha/architecture.json";

/// How to render the report on stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Text,
    Json,
    Markdown,
}

impl Format {
    /// Parses `--format`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "md" | "markdown" => Some(Self::Markdown),
            _ => None,
        }
    }
}

/// What the command was asked to do.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// An external workspace's `Cargo.toml`.
    pub manifest_path: Option<PathBuf>,
    /// An external rules file. Defaults to `rha-crates.toml` at the root.
    pub rules_path: Option<PathBuf>,
    pub format: Format,
    /// Accepted and reported as `not_run` unless `[transitive] enabled`.
    pub transitive: bool,
}

/// Runs the check and returns the exit code.
///
/// # Errors
/// Returns an error only for a failure that is neither a finding nor an
/// environment problem; both of those map to an exit code instead.
pub fn run(root: &Path, options: &Options) -> Result<i32> {
    let rules_path = options
        .rules_path
        .clone()
        .unwrap_or_else(|| root.join(RULES_PATH));
    let (mut rules, rules_digest) = match Rules::load(&rules_path) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("xtask architecture: {e}");
            return Ok(EXIT_USAGE);
        }
    };

    if options.transitive && !rules.transitive.enabled {
        // The flag is accepted, and turning it on is the rules file's
        // decision, not the command line's. Say so rather than silently
        // ignoring the flag or silently widening what is checked.
        eprintln!(
            "xtask architecture: --transitive was given but [transitive] enabled is false in {}; \
             transitive rules stay not_run",
            rules_path.display()
        );
        rules.transitive.enabled = false;
    }

    let graph = match metadata::load(root, options.manifest_path.as_deref()) {
        Ok(graph) => graph,
        Err(e) => {
            // An environment failure is never a pass (plan §5).
            eprintln!("xtask architecture: environment failure: {e}");
            write_report(
                root,
                &serde_json::json!({
                    "schema_version": 1,
                    "tool": { "name": "xtask architecture", "version": env!("CARGO_PKG_VERSION") },
                    "subject": { "workspace_root": root.display().to_string() },
                    "summary": {
                        "errors": null, "warnings": null,
                        "outcome": "failed", "error_class": "tool_error",
                        "reason": e.to_string(),
                    },
                }),
            )?;
            return Ok(EXIT_ENVIRONMENT);
        }
    };

    let mut outcome = check::check(&graph, &rules);
    clippy_template_rule(root, &graph, &mut outcome);

    let manifest = options
        .manifest_path
        .as_ref()
        .map(|p| p.display().to_string());
    let json = report::json(
        &graph,
        &outcome,
        &rules_path.display().to_string(),
        &rules_digest,
        manifest.as_deref(),
    );
    write_report(root, &json)?;

    match options.format {
        Format::Text => print!("{}", report::text(&graph, &outcome)),
        Format::Markdown => print!("{}", report::markdown(&graph, &outcome)),
        Format::Json => {
            let mut text = serde_json::to_string_pretty(&json)
                .context(|| "serializing the report".to_owned())?;
            text.push('\n');
            print!("{text}");
        }
    }
    Ok(report::exit_code(outcome.errors()))
}

/// `effect.core_clippy_template` (CHG-001), evaluated here because this is
/// the command that knows which crates are cores.
///
/// Until CHG-005 there is no core crate, so this rule examines nothing. That
/// is not the same as it passing over a core crate, and the limitation says
/// which it is.
fn clippy_template_rule(
    root: &Path,
    graph: &crate::graph::model::CrateGraph,
    outcome: &mut check::Outcome,
) {
    let cores: Vec<clippy_template::CoreCrate> = graph
        .crates
        .iter()
        .filter(|c| c.role == Some(crate::graph::model::Role::Core))
        .filter_map(|c| {
            c.manifest_path
                .parent()
                .map(|dir| clippy_template::CoreCrate {
                    name: c.name.clone(),
                    dir: dir.to_path_buf(),
                })
        })
        .collect();
    if cores.is_empty() {
        outcome.limitations.push(format!(
            "{}: no crate is classified core at this revision, so the rule examined nothing. \
             It is not a pass over a core crate.",
            clippy_template::RULE_ID
        ));
        return;
    }
    let template_path = root.join(clippy_template::TEMPLATE_PATH);
    let Ok(template) = std::fs::read(&template_path) else {
        outcome.limitations.push(format!(
            "{}: not evaluated, {} is unreadable",
            clippy_template::RULE_ID,
            template_path.display()
        ));
        return;
    };
    for finding in clippy_template::check(&template, &cores) {
        let detail = finding
            .detail
            .clone()
            .unwrap_or_else(|| format!("{:?}", finding.problem));
        outcome.findings.push(check::Finding {
            rule: clippy_template::RULE_ID,
            severity: check::Severity::Error,
            from: finding.krate.clone(),
            to: None,
            kind: None,
            manifest_path: finding.path.clone(),
            witness: Some(check::Witness {
                edge: format!(
                    "expected sha256:{} at {}",
                    finding.expected_sha256, finding.path
                ),
                declared_in: finding.path.clone(),
                optional: false,
                target: finding.actual_sha256.clone().map(|a| format!("sha256:{a}")),
                rename: None,
            }),
            message: format!(
                "{}: the core crate's Clippy configuration does not match the template: {detail}",
                finding.krate
            ),
        });
    }
}

fn write_report(root: &Path, json: &serde_json::Value) -> Result<()> {
    let dir = root.join(RUN_DIR);
    std::fs::create_dir_all(&dir).context(|| format!("creating {}", dir.display()))?;
    let path = root.join(REPORT_PATH);
    let mut text =
        serde_json::to_string_pretty(json).context(|| "serializing the report".to_owned())?;
    text.push('\n');
    std::fs::write(&path, text).context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_flag_accepts_what_plan_section_5_lists() {
        assert_eq!(Format::parse("text"), Some(Format::Text));
        assert_eq!(Format::parse("json"), Some(Format::Json));
        assert_eq!(Format::parse("md"), Some(Format::Markdown));
        assert_eq!(Format::parse("markdown"), Some(Format::Markdown));
        assert_eq!(Format::parse("yaml"), None);
    }

    #[test]
    fn the_exit_codes_are_the_ones_the_lane_relies_on() {
        // An environment failure must never share a code with success.
        assert_ne!(EXIT_ENVIRONMENT, 0);
        assert_ne!(EXIT_USAGE, 0);
        assert_eq!((EXIT_USAGE, EXIT_ENVIRONMENT, EXIT_NOT_RUN), (2, 3, 4));
    }
}
