//! `cargo xtask architecture`: crate-graph and module-graph checks (spec §6.13).
//!
//! Not implemented yet: CHG-003 (W3) delivers the crate-graph checker and
//! CHG-007 (W7) the module-graph check. Until then the command writes a report
//! whose outcome is `not_run` and exits with [`EXIT_NOT_RUN`], so nothing that
//! reads either the exit status or the report can mistake it for a pass.

use std::path::Path;

use serde_json::json;

use crate::error::{Context as _, Result};
use crate::lanes::RUN_DIR;

/// Exit status meaning "the check did not run; see the report".
pub const EXIT_NOT_RUN: i32 = 4;

/// Where the architecture report is written.
pub const REPORT_PATH: &str = "target/rha/architecture.json";

/// Writes the `not_run` report and returns [`EXIT_NOT_RUN`].
///
/// # Errors
/// Fails if the report cannot be written.
pub fn run(root: &Path) -> Result<i32> {
    let reason = "not_implemented: the crate-graph checker arrives in CHG-003 (W3), the module-graph check in CHG-007 (W7)";
    let report = json!({
        "schema_version": 1,
        "tool": { "name": "xtask architecture", "version": env!("CARGO_PKG_VERSION") },
        "subject": { "workspace_root": root.display().to_string() },
        "classification": [],
        "findings": [],
        "module_checks": [],
        "limitations": ["no rules are evaluated at this version"],
        "summary": { "errors": null, "warnings": null, "outcome": "not_run", "reason": reason },
    });
    let dir = root.join(RUN_DIR);
    std::fs::create_dir_all(&dir).context(|| format!("creating {}", dir.display()))?;
    let path = root.join(REPORT_PATH);
    let mut text =
        serde_json::to_string_pretty(&report).context(|| "serializing the report".to_owned())?;
    text.push('\n');
    std::fs::write(&path, text).context(|| format!("writing {}", path.display()))?;
    eprintln!("xtask architecture: not run ({reason}); report: {REPORT_PATH}");
    Ok(EXIT_NOT_RUN)
}
