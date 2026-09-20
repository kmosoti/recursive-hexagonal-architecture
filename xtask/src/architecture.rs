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
    /// Accepted for the L2 interface. This version has no transitive
    /// evaluator, so the flag prints a notice, and `[transitive] enabled =
    /// true` in the rules file is refused with exit 2 rather than run as if
    /// it had been honoured.
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
    let (rules, rules_digest) = match Rules::load(&rules_path) {
        Ok(loaded) => loaded,
        Err(e) => return failure(root, options, &rules_path, "config_error", &e.to_string()),
    };

    if rules.transitive.enabled {
        // Requested and unimplemented is a configuration this version cannot
        // honour, and 2 is the exit code for that (plan §5). Running the
        // direct rules anyway and exiting 0 reported a clean summary for a
        // check that was asked for and never performed (review finding on
        // pull request 6, CHG-003.1). The lane records exit 2 as failed with
        // error_class config_error, never as passed.
        return failure(
            root,
            options,
            &rules_path,
            "config_error",
            &format!(
                "[transitive] enabled = true in {}, but this version has no transitive evaluator",
                rules_path.display()
            ),
        );
    }
    if options.transitive {
        // The flag is accepted so the L2 interface is stable; what it asks
        // for is not implemented, and the report's limitations say so.
        eprintln!(
            "xtask architecture: --transitive was given; transitive rules are not implemented \
             in this version and stay not evaluated (see the report's limitations)"
        );
    }

    let graph = match metadata::load(root, options.manifest_path.as_deref()) {
        Ok(graph) => graph,
        Err(metadata::LoadError::Configuration(e)) => {
            return failure(root, options, &rules_path, "config_error", &e.to_string());
        }
        Err(metadata::LoadError::Environment(e)) => {
            return failure(root, options, &rules_path, "tool_error", &e.to_string());
        }
    };

    let mut outcome = check::check(&graph, &rules);
    clippy_template_rule(root, &mut outcome);

    let manifest = options
        .manifest_path
        .as_ref()
        .map(|p| p.display().to_string());
    let json = report::json(
        &graph,
        &outcome,
        &report::tool_identity(root),
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
fn clippy_template_rule(root: &Path, outcome: &mut check::Outcome) {
    // The cores come from the classification the check recorded, not from
    // the graph as loaded: `metadata::build` leaves every role `None` and
    // `check::check` classifies a clone, so reading the loaded graph here
    // found no core in any workspace and the rule never examined a crate
    // (review finding on 920cca6, CHG-003.3).
    let cores: Vec<clippy_template::CoreCrate> = outcome
        .classification
        .iter()
        .filter(|c| c.role == Some(crate::graph::model::Role::Core))
        .filter_map(|c| {
            Path::new(&c.manifest_path)
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
            details: std::collections::BTreeMap::new(),
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

fn failure(
    root: &Path,
    options: &Options,
    rules_path: &Path,
    error_class: &str,
    reason: &str,
) -> Result<i32> {
    let json = serde_json::json!({
        "schema_version": 1,
        "tool": report::tool_identity(root),
        "subject": {
            "workspace_root": options.manifest_path.is_none().then(|| root.display().to_string()),
            "manifest_path": options.manifest_path.as_ref().map(|p| p.display().to_string()),
            "rules_path": rules_path.display().to_string(),
        },
        "summary": {
            "errors": null,
            "warnings": null,
            "outcome": "failed",
            "error_class": error_class,
            "reason": reason,
        },
    });
    write_report(root, &json)?;
    if options.format == Format::Json {
        let mut text = serde_json::to_string_pretty(&json)
            .context(|| "serializing the failure report".to_owned())?;
        text.push('\n');
        print!("{text}");
    } else {
        eprintln!("xtask architecture: {}", json["summary"]["reason"]);
    }
    let code = if error_class == "tool_error" {
        EXIT_ENVIRONMENT
    } else {
        EXIT_USAGE
    };
    Ok(code)
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
    fn transitive_enabled_is_refused_before_anything_runs() {
        let dir = std::env::temp_dir().join(format!("rha-transitive-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let rules = dir.join("rha-crates.toml");
        std::fs::write(
            &rules,
            "schema_version = 1\n[classification]\n[transitive]\nenabled = true\n",
        )
        .expect("write rules");
        let options = Options {
            rules_path: Some(rules),
            ..Options::default()
        };
        // `dir` has no Cargo.toml. If the refusal did not come first, this
        // would be an environment failure (3), not a configuration error (2).
        let code = run(&dir, &options).expect("run returns a code");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(code, EXIT_USAGE);
    }

    #[test]
    fn the_exit_codes_are_the_ones_the_lane_relies_on() {
        // An environment failure must never share a code with success.
        assert_ne!(EXIT_ENVIRONMENT, 0);
        assert_ne!(EXIT_USAGE, 0);
        assert_eq!((EXIT_USAGE, EXIT_ENVIRONMENT, EXIT_NOT_RUN), (2, 3, 4));
    }
}
