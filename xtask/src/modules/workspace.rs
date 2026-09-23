//! Applies declared module checks to a completed crate-graph report.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::graph::model::CrateGraph;

use super::{extract, rules};

/// Applies every declared composite check and combines its observations with
/// the crate-level report.
///
/// # Errors
///
/// A declared source or rules file that cannot be extracted or checked is a
/// configuration error. The error includes the affected crate.
pub fn apply(graph: &CrateGraph, report: &mut Value) -> Result<(), String> {
    let mut checks = Vec::new();

    for node in &graph.crates {
        let Some(composite) = node.composite.as_ref() else {
            checks.push(json!({
                "crate": node.name,
                "required": false,
                "rules_path": null,
                "rules_digest": null,
                "outcome": "not_run",
                "reason": "not_declared_composite: no module rules obligation",
            }));
            continue;
        };

        let crate_root = node
            .manifest_path
            .parent()
            .ok_or_else(|| {
                format!(
                    "crate {} manifest {} has no parent directory",
                    node.name,
                    node.manifest_path.display()
                )
            })?
            .to_path_buf();
        let source_path = resolve_path(&crate_root, &composite.source);
        let rules_path = resolve_path(&crate_root, Path::new(&composite.rules));

        let extracted = extract::extract(
            &crate_root,
            &source_path,
            &composite.crate_name,
            &composite.edition,
            &composite.externals,
        )
        .map_err(|error| {
            format!(
                "crate {}: extracting declared module source {}: {error}",
                node.name,
                source_path.display()
            )
        })?;

        let test_edges = extracted.edges.iter().filter(|edge| edge.test_only).count();
        let checked = rules::check(&crate_root, &composite.crate_name, &rules_path, &extracted)
            .map_err(|error| {
                format!(
                    "crate {}: checking module rules {}: {error}",
                    node.name,
                    rules_path.display()
                )
            })?;

        let module_errors = checked
            .findings
            .iter()
            .any(|finding| finding["severity"].as_str() == Some("error"));

        let mut limitations = vec![tagged(
            json!({
                "code": "source_approximation",
                "detail": "module extraction is source-based; heuristic paths and unexpanded macros or include! remain limitations",
            }),
            &composite.crate_name,
            &node.name,
        )];
        limitations.extend(
            checked
                .limitations
                .into_iter()
                .map(|limitation| tagged(limitation, &composite.crate_name, &node.name)),
        );
        for limitation in &limitations {
            push(report, "limitations", limitation.clone())?;
        }

        for finding in checked.findings {
            push(
                report,
                "findings",
                tagged(finding, &composite.crate_name, &node.name),
            )?;
        }
        for edge in checked.test_edges {
            let mut edge = tagged(edge, &composite.crate_name, &node.name);
            if let Some(object) = edge.as_object_mut() {
                object
                    .entry("severity".to_owned())
                    .or_insert_with(|| json!("note"));
            }
            push(report, "test_edges", edge)?;
        }
        for edge in checked.module_edges {
            push(
                report,
                "module_edges",
                tagged(edge, &composite.crate_name, &node.name),
            )?;
        }
        checks.push(json!({
            "crate": node.name,
            "required": true,
            "rules_path": checked.rules_path,
            "rules_digest": checked.rules_digest,
            "outcome": if module_errors { "failed" } else { "passed" },
            "reason": null,
            "source": source_path,
            "extraction": {
                "scope": "rules configured by the declared rules file; source-based extraction is approximate",
                "limits": limitations,
                "counts": {
                    "modules": extracted.modules.len(),
                    "edges": extracted.edges.len(),
                    "test_edges": test_edges,
                },
            },
        }));
    }

    report["module_checks"] = json!(checks);
    recompute_summary(report)
}

fn resolve_path(crate_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate_root.join(path)
    }
}

fn tagged(mut value: Value, crate_name: &str, package: &str) -> Value {
    let Some(object) = value.as_object_mut() else {
        return json!({
            "crate": crate_name,
            "package": package,
            "value": value,
        });
    };
    object
        .entry("crate".to_owned())
        .or_insert_with(|| json!(crate_name));
    object
        .entry("package".to_owned())
        .or_insert_with(|| json!(package));
    value
}

fn push(report: &mut Value, key: &str, value: Value) -> Result<(), String> {
    report
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("architecture report has no {key} array"))?
        .push(value);
    Ok(())
}

fn recompute_summary(report: &mut Value) -> Result<(), String> {
    let findings = report
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "architecture report has no findings array".to_owned())?;
    let errors = findings
        .iter()
        .filter(|finding| finding["severity"].as_str() == Some("error"))
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding["severity"].as_str() == Some("warning"))
        .count();
    let summary = report
        .get_mut("summary")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "architecture report has no summary object".to_owned())?;
    summary.insert("errors".to_owned(), json!(errors));
    summary.insert("warnings".to_owned(), json!(warnings));
    summary.insert(
        "outcome".to_owned(),
        json!(if errors == 0 { "passed" } else { "failed" }),
    );
    Ok(())
}
