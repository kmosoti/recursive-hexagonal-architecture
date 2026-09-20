//! The architecture report: JSON, text, and markdown (plan §5).
//!
//! The JSON is always written to `target/rha/architecture.json`, whatever the
//! `--format`, so the lane has a machine-readable artifact even when a human
//! asked for text.

use std::path::Path;

use serde_json::{Value, json};

use crate::graph::check::{Outcome, Severity};
use crate::graph::model::{CrateGraph, MetadataMode};

/// Identity of the checker that produced a report: the checkout it was built
/// from, read at run time the way `xtask ci` reads it for the evidence
/// record's producer, so the fact has one owner and one definition of dirty
/// (CHG-003.2). `cargo xtask` rebuilds the binary from the working tree
/// before every run, so the running checker is that checkout. `root` is the
/// tool's own workspace; the subject under `--manifest-path` never reaches
/// this. `unknown` and `null` when git cannot answer, never a guess.
#[must_use]
pub fn tool_identity(root: &Path) -> Value {
    let identity = crate::util::git_identity(root);
    json!({
        "name": "xtask architecture",
        "version": env!("CARGO_PKG_VERSION"),
        "git_rev": identity.as_ref().map_or("unknown", |i| i.revision.as_str()),
        "git_dirty": identity.as_ref().map(|i| i.dirty),
    })
}

/// What the report says about the module-level checks, which do not exist
/// until CHG-007. `not_run` with a reason, never an empty list of findings
/// that reads like a pass (§11.4).
#[must_use]
pub fn module_checks(graph: &CrateGraph) -> Value {
    graph
        .crates
        .iter()
        .map(|c| {
            json!({
                "crate": c.name,
                "rules_path": c.manifest_path.parent()
                    .map(|d| d.join("rha-modules.toml").display().to_string()),
                "outcome": "not_run",
                "reason": "not_implemented: the module-graph check arrives in CHG-007 (W7)",
            })
        })
        .collect()
}

/// Which exit code an outcome deserves (plan §5): 0 no error findings, 1
/// error findings. 2 and 3 are raised by the caller for usage and
/// environment failures.
#[must_use]
pub const fn exit_code(errors: usize) -> i32 {
    if errors == 0 { 0 } else { 1 }
}

/// The JSON report.
#[must_use]
pub fn json(
    graph: &CrateGraph,
    outcome: &Outcome,
    tool: &Value,
    rules_path: &str,
    rules_digest: &str,
    manifest_path: Option<&str>,
) -> Value {
    let (normal, dev, build) = graph.edge_counts();
    let errors = outcome.errors();
    json!({
        "schema_version": 1,
        "tool": tool,
        "subject": {
            "workspace_root": graph.workspace_root.display().to_string(),
            "manifest_path": manifest_path,
            "metadata_mode": match graph.mode {
                MetadataMode::NoDeps => "no_deps",
                MetadataMode::Resolved => "resolved",
            },
            "rules_path": rules_path,
            "rules_digest": format!("sha256:{rules_digest}"),
        },
        "classification": outcome.classification,
        "edges_examined": { "normal": normal, "dev": dev, "build": build },
        "findings": outcome.findings.iter().map(finding_json).collect::<Vec<_>>(),
        "harness_edges": outcome.harness_edges.iter().map(finding_json).collect::<Vec<_>>(),
        "module_checks": module_checks(graph),
        "limitations": outcome.limitations,
        "summary": {
            "crates": graph.crates.len(),
            "errors": errors,
            "warnings": outcome.warnings(),
            "outcome": if errors == 0 { "passed" } else { "failed" },
        },
    })
}

fn finding_json(finding: &crate::graph::check::Finding) -> Value {
    let mut value = json!(finding);
    // `from` names the subject of both edge and per-crate diagnostics.
    value["crate"] = json!(finding.from);
    value
}

/// The text report, in the shape plan §5 gives:
/// `error[dir.core_to_adapter]: site -> adapter-html (normal) declared in …`
#[must_use]
pub fn text(graph: &CrateGraph, outcome: &Outcome) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let crates: Vec<String> = outcome
        .classification
        .iter()
        .map(|c| {
            let role = c.role.map_or("unclassified", |r| r.as_str());
            let source = c
                .role_source
                .map_or_else(|| "-".to_owned(), |s| format!("{s:?}").to_lowercase());
            format!("{} ({role}, {source})", c.name)
        })
        .collect();
    let _ = writeln!(
        out,
        "crates: {} ({})",
        graph.crates.len(),
        crates.join(", ")
    );
    let (normal, dev, build) = graph.edge_counts();
    let _ = writeln!(
        out,
        "edges examined: {normal} normal, {dev} dev, {build} build"
    );

    for finding in &outcome.findings {
        let level = match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };
        let _ = write!(out, "{level}[{}]: ", finding.rule);
        match (&finding.to, finding.kind) {
            (Some(to), Some(kind)) => {
                let _ = write!(out, "{} -> {to} ({kind})", finding.from);
            }
            _ => {
                let _ = write!(out, "{}", finding.from);
            }
        }
        if let Some(witness) = &finding.witness {
            let _ = write!(
                out,
                " declared in {} {}",
                finding.manifest_path, witness.declared_in
            );
        }
        let _ = writeln!(out, ": {}", finding.message);
    }

    for edge in &outcome.harness_edges {
        let _ = writeln!(
            out,
            "harness_edges[{}]: {}: {}",
            edge.rule,
            edge.witness
                .as_ref()
                .map_or_else(String::new, |w| w.edge.clone()),
            edge.message
        );
    }
    for limitation in &outcome.limitations {
        let _ = writeln!(out, "limitation: {limitation}");
    }
    let _ = writeln!(
        out,
        "module_checks: not_run (not_implemented: CHG-007)\nsummary: {} error(s), {} warning(s)",
        outcome.errors(),
        outcome.warnings()
    );
    out
}

/// The markdown report, for pasting into a review.
#[must_use]
pub fn markdown(graph: &CrateGraph, outcome: &Outcome) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("# Architecture report\n\n## Classification\n\n");
    let _ = writeln!(out, "| Crate | Role | Source |\n| --- | --- | --- |");
    for c in &outcome.classification {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} |",
            c.name,
            c.role.map_or("**unclassified**", |r| r.as_str()),
            c.role_source
                .map_or_else(|| "-".to_owned(), |s| format!("{s:?}").to_lowercase())
        );
    }
    let _ = writeln!(out, "\n## Findings\n");
    if outcome.findings.is_empty() {
        let _ = writeln!(out, "None.\n");
    } else {
        let _ = writeln!(
            out,
            "| Severity | Rule | Subject | Message |\n| --- | --- | --- | --- |"
        );
        for f in &outcome.findings {
            let subject =
                f.to.as_ref()
                    .map_or_else(|| f.from.clone(), |to| format!("`{}` → `{to}`", f.from));
            let _ = writeln!(
                out,
                "| {:?} | `{}` | {subject} | {} |",
                f.severity, f.rule, f.message
            );
        }
    }
    let _ = writeln!(
        out,
        "\n## Module checks\n\n`not_run`: not_implemented, the module-graph check arrives in CHG-007 (W7).\n"
    );
    if !outcome.limitations.is_empty() {
        let _ = writeln!(out, "## Limitations\n");
        for l in &outcome.limitations {
            let _ = writeln!(out, "- {l}");
        }
    }
    let _ = writeln!(
        out,
        "\n**Summary:** {} crate(s), {} error(s), {} warning(s).",
        graph.crates.len(),
        outcome.errors(),
        outcome.warnings()
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::model::{CrateNode, Role, RoleSource};
    use std::path::PathBuf;

    fn graph() -> CrateGraph {
        CrateGraph {
            workspace_root: PathBuf::from("/w"),
            crates: vec![CrateNode {
                name: "xtask".to_owned(),
                manifest_path: PathBuf::from("/w/xtask/Cargo.toml"),
                role: Some(Role::Tool),
                role_source: Some(RoleSource::List),
                declared_role: None,
                implements: Vec::new(),
                has_build_script: false,
            }],
            edges: Vec::new(),
            mode: MetadataMode::NoDeps,
        }
    }

    #[test]
    fn a_clean_run_exits_zero_and_a_finding_exits_one() {
        assert_eq!(exit_code(0), 0);
        assert_eq!(exit_code(1), 1);
        assert_eq!(exit_code(7), 1);
    }

    #[test]
    fn module_checks_report_not_run_with_a_reason_never_an_empty_pass() {
        let checks = module_checks(&graph());
        assert_eq!(checks.as_array().expect("an array").len(), 1);
        assert_eq!(checks[0]["outcome"], "not_run");
        assert!(
            checks[0]["reason"]
                .as_str()
                .expect("a reason")
                .contains("CHG-007")
        );
    }

    #[test]
    fn the_json_report_carries_the_plan_section_5_fields() {
        let outcome = Outcome {
            classification: vec![crate::graph::check::ClassifiedCrate {
                name: "xtask".to_owned(),
                role: Some(Role::Tool),
                role_source: Some(RoleSource::List),
                manifest_path: "/w/xtask/Cargo.toml".to_owned(),
            }],
            ..Outcome::default()
        };
        let tool = tool_identity(Path::new(env!("CARGO_MANIFEST_DIR")));
        let report = json(
            &graph(),
            &outcome,
            &tool,
            "rha-crates.toml",
            &"a".repeat(64),
            None,
        );
        for key in [
            "schema_version",
            "tool",
            "subject",
            "classification",
            "edges_examined",
            "findings",
            "module_checks",
            "limitations",
            "summary",
        ] {
            assert!(report.get(key).is_some(), "missing {key}");
        }
        assert_eq!(report["subject"]["metadata_mode"], "no_deps");
        assert_eq!(report["summary"]["errors"], 0);
        assert_eq!(report["summary"]["outcome"], "passed");
        assert!(report["tool"]["name"].is_string());
        assert!(report["tool"]["version"].is_string());
        assert!(report["tool"]["git_rev"].is_string());
        assert!(report["tool"].get("git_dirty").is_some());
        assert!(
            report["subject"]["rules_digest"]
                .as_str()
                .expect("a digest")
                .starts_with("sha256:")
        );
    }

    #[test]
    fn the_text_report_names_the_crate_and_its_classification() {
        let outcome = Outcome {
            classification: vec![crate::graph::check::ClassifiedCrate {
                name: "xtask".to_owned(),
                role: Some(Role::Tool),
                role_source: Some(RoleSource::List),
                manifest_path: "/w/xtask/Cargo.toml".to_owned(),
            }],
            ..Outcome::default()
        };
        let text = text(&graph(), &outcome);
        assert!(text.contains("crates: 1 (xtask (tool, list))"), "{text}");
        assert!(text.contains("module_checks: not_run"), "{text}");
    }
}
