//! Module-only architecture reports for frozen single-crate fixtures.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::graph::report as graph_report;
use crate::metadata;
use crate::util;

use super::{extract, rules, workspace};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleError {
    pub class: &'static str,
    pub message: String,
}

impl ModuleError {
    fn config(message: impl Into<String>) -> Self {
        Self {
            class: "config_error",
            message: message.into(),
        }
    }

    fn tool(message: impl Into<String>) -> Self {
        Self {
            class: "tool_error",
            message: message.into(),
        }
    }
}

impl fmt::Display for ModuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ModuleError {}

pub fn report(root: &Path, manifest: &Path, module_rules: &Path) -> Result<Value, ModuleError> {
    let manifest = canonical_input(root, manifest, "manifest")?;
    let module =
        metadata::load_single_module(root, &manifest, module_rules).map_err(map_load_error)?;
    let manifest_digest = digest(&manifest, "manifest")?;

    let crate_root = module
        .manifest_path
        .parent()
        .ok_or_else(|| ModuleError::config("selected package manifest has no parent directory"))?;
    let crate_root = fs::canonicalize(crate_root).map_err(|error| {
        ModuleError::config(format!(
            "failed to canonicalize package root {}: {error}",
            crate_root.display()
        ))
    })?;
    let source = fs::canonicalize(&module.source.source).map_err(|error| {
        ModuleError::config(format!(
            "failed to canonicalize library source {}: {error}",
            module.source.source.display()
        ))
    })?;
    if !source.starts_with(&crate_root) {
        return Err(ModuleError::config(
            "library source is outside the selected crate root",
        ));
    }
    let source_digest = digest(&source, "source")?;
    let module_rules = canonical_input(root, module_rules, "module rules")?;

    let extracted = extract::extract(
        &crate_root,
        &source,
        &module.source.crate_name,
        &module.source.edition,
        &module.source.externals,
    )
    .map_err(ModuleError::config)?;
    let source_files =
        workspace::source_files(&crate_root, &extracted).map_err(ModuleError::config)?;
    let checked = rules::check(
        &crate_root,
        &module.source.crate_name,
        &module_rules,
        &extracted,
    )
    .map_err(ModuleError::config)?;

    let package_name = module.package.as_str();
    let crate_name = module.source.crate_name.as_str();
    let mut limitations = vec![tagged(
        json!({
            "code": "source_approximation",
            "detail": "module extraction is source-based; heuristic paths and unexpanded macros or include! remain limitations",
        }),
        package_name,
        crate_name,
    )];
    limitations.extend(
        checked
            .limitations
            .iter()
            .cloned()
            .map(|limitation| tagged(limitation, package_name, crate_name)),
    );

    let findings: Vec<Value> = checked
        .findings
        .iter()
        .cloned()
        .map(|finding| tagged(finding, package_name, crate_name))
        .collect();
    let test_edges: Vec<Value> = checked
        .test_edges
        .iter()
        .cloned()
        .map(|edge| {
            let mut edge = tagged(edge, package_name, crate_name);
            if let Some(object) = edge.as_object_mut() {
                object
                    .entry("severity".to_owned())
                    .or_insert_with(|| json!("note"));
            }
            edge
        })
        .collect();
    let module_edges: Vec<Value> = checked
        .module_edges
        .iter()
        .cloned()
        .map(|edge| tagged(edge, package_name, crate_name))
        .collect();

    let errors = findings
        .iter()
        .filter(|finding| finding["severity"] == "error")
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding["severity"] == "warning")
        .count();
    let rules_digest = format!("sha256:{}", checked.rules_digest);
    let module_outcome = if errors == 0 { "passed" } else { "failed" };
    let workspace_root = crate_root.display().to_string();

    Ok(json!({
        "schema_version": 1,
        "tool": graph_report::tool_identity(root),
        "subject": {
            "mode": "module_only",
            "workspace_root": workspace_root,
            "manifest_path": manifest.display().to_string(),
            "manifest_sha256": format!("sha256:{manifest_digest}"),
            "package_name": package_name,
            "source": source.display().to_string(),
            "source_sha256": format!("sha256:{source_digest}"),
            "source_files_sha256": source_files,
            "crate_name": crate_name,
            "edition": module.source.edition,
            "rules_path": checked.rules_path,
            "rules_digest": rules_digest,
            "metadata_mode": "no_deps",
        },
        "module_checks": [{
            "package": package_name,
            "crate": crate_name,
            "required": true,
            "rules_path": checked.rules_path,
            "rules_digest": rules_digest,
            "outcome": module_outcome,
            "reason": null,
            "source": source.display().to_string(),
            "source_files_sha256": source_files,
            "extraction": {
                "scope": "rules configured by the supplied module rules file; source-based extraction is approximate",
                "counts": {
                    "modules": extracted.modules.len(),
                    "edges": extracted.edges.len(),
                    "test_edges": extracted.edges.iter().filter(|edge| edge.test_only).count(),
                },
                "limits": limitations,
            },
        }],
        "findings": findings,
        "test_edges": test_edges,
        "module_edges": module_edges,
        "limitations": limitations,
        "summary": {
            "crates": 1,
            "errors": errors,
            "warnings": warnings,
            "outcome": module_outcome,
        },
    }))
}

fn map_load_error(error: metadata::LoadError) -> ModuleError {
    match error {
        metadata::LoadError::Configuration(error) => ModuleError::config(error.to_string()),
        metadata::LoadError::Environment(error) => ModuleError::tool(error.to_string()),
    }
}

fn canonical_input(root: &Path, path: &Path, label: &str) -> Result<PathBuf, ModuleError> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    fs::canonicalize(&path).map_err(|error| {
        ModuleError::config(format!(
            "failed to canonicalize {label} {}: {error}",
            path.display()
        ))
    })
}

fn digest(path: &Path, label: &str) -> Result<String, ModuleError> {
    fs::read(path)
        .map(|bytes| util::sha256_hex(&bytes))
        .map_err(|error| {
            ModuleError::config(format!(
                "failed to read {label} {}: {error}",
                path.display()
            ))
        })
}

fn tagged(mut value: Value, package: &str, crate_name: &str) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert("package".to_owned(), json!(package));
        object.insert("crate".to_owned(), json!(crate_name));
        value
    } else {
        json!({
            "package": package,
            "crate": crate_name,
            "value": value,
        })
    }
}
