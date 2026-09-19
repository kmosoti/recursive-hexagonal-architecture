//! The tool inventory (`rha-baseline.json`) and installed-version probes.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Context as _, Result};
use crate::util::{command, first_version};

pub const BASELINE_PATH: &str = "rha-baseline.json";

/// The pseudo-tool name for checks that `xtask` itself implements.
pub const SELF_TOOL: &str = "xtask";

#[derive(Debug, Deserialize)]
pub struct Baseline {
    pub schema_version: u32,
    pub about: String,
    pub toolchain_file: String,
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub status: String,
    pub used_by: Vec<String>,
    pub version_command: Vec<String>,
    pub required: Option<String>,
    pub install: String,
    pub ci_install: Option<String>,
    pub bootstrap_observed: String,
}

impl Baseline {
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&ToolSpec> {
        self.tools.iter().find(|t| t.name == name)
    }
}

/// Reads `rha-baseline.json` under `root`.
///
/// # Errors
/// Fails if the file cannot be read or parsed.
pub fn load(root: &Path) -> Result<Baseline> {
    let path = root.join(BASELINE_PATH);
    let text = std::fs::read_to_string(&path).context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).context(|| format!("parsing {}", path.display()))
}

/// What a version probe found for one tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Probe {
    /// Installed at the required version.
    Ready { version: String },
    /// The probe command could not run or exited nonzero.
    Missing { detail: String },
    /// Installed, but not at the required version.
    Mismatch { installed: String, required: String },
    /// The inventory has no entry for this tool, or no required version.
    Unlisted,
}

/// Probes `spec` by running its version command in `root`.
#[must_use]
pub fn probe(root: &Path, spec: &ToolSpec) -> Probe {
    let Some(required) = spec.required.as_deref() else {
        return Probe::Unlisted;
    };
    let Some((program, args)) = spec.version_command.split_first() else {
        return Probe::Unlisted;
    };
    let output = match command(program).args(args).current_dir(root).output() {
        Ok(output) => output,
        Err(e) => {
            return Probe::Missing {
                detail: format!("`{}`: {e}", spec.version_command.join(" ")),
            };
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Probe::Missing {
            detail: format!(
                "`{}` exited with {}: {}",
                spec.version_command.join(" "),
                output.status,
                stderr.lines().next().unwrap_or("").trim()
            ),
        };
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    compare(first_version(&stdout), required)
}

/// Compares an observed version token with the required one.
#[must_use]
pub fn compare(observed: Option<&str>, required: &str) -> Probe {
    match observed {
        Some(v) if v == required => Probe::Ready {
            version: v.to_owned(),
        },
        Some(v) => Probe::Mismatch {
            installed: v.to_owned(),
            required: required.to_owned(),
        },
        None => Probe::Missing {
            detail: "version output has no MAJOR.MINOR.PATCH token".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_version_is_ready_and_anything_else_is_not() {
        assert_eq!(
            compare(Some("0.9.145"), "0.9.145"),
            Probe::Ready {
                version: "0.9.145".into()
            }
        );
        assert_eq!(
            compare(Some("0.9.144"), "0.9.145"),
            Probe::Mismatch {
                installed: "0.9.144".into(),
                required: "0.9.145".into()
            }
        );
        assert!(matches!(compare(None, "0.9.145"), Probe::Missing { .. }));
    }
}
