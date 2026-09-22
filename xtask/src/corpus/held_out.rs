//! `cargo xtask corpus held-out`: the accepted checker on Kennedy's private
//! cases, with no agent reading them (DP-1.1b as amended in CHG-004.6).
//!
//! What it prints is bounded on purpose: opaque case ids, exit statuses,
//! finding counts, the checker's identity and the commitment check. Raw
//! reports and the map from opaque id to private path go to a private
//! directory outside the repository. Nothing is graded: an observed exit
//! status is not a claim that Kennedy's expectation passed. He compares the
//! observations with his expectations and records the per-case outcomes in
//! the acceptance record.

use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

use crate::cli::HeldOutArgs;
use crate::error::{Context as _, Error, Result};
use crate::util::{UtcTime, command, git_identity, sha256_file, sha256_hex};

/// The flags DP-1.1b records for the commitment. A directory is verified by
/// re-creating exactly this stream and hashing it.
pub const COMMITMENT_TAR_FLAGS: [&str; 5] = [
    "--sort=name",
    "--mtime=2026-09-20",
    "--owner=0",
    "--group=0",
    "--numeric-owner",
];

/// The ledger row whose commitment binds a kind of held-out case: DP-1.1b
/// for crate workspaces, DP-1.4 for markdown sites (review finding 6).
#[must_use]
pub fn commitment_row(kind: &str) -> &'static str {
    if kind == "check" { "DP-1.4" } else { "DP-1.1b" }
}

/// The commitment recorded on a ledger row.
///
/// # Errors
/// Fails if the ledger cannot be read or the row carries no commitment.
pub fn commitment(root: &Path, row_id: &str) -> Result<String> {
    let text = std::fs::read_to_string(root.join(".rha/decisions.toml"))
        .context(|| "reading the decision ledger".to_owned())?;
    let ledger: toml::Value =
        toml::from_str(&text).context(|| "parsing the decision ledger".to_owned())?;
    ledger
        .get("decision")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("id").and_then(toml::Value::as_str) == Some(row_id))
        .and_then(|row| row.get("commitment"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::new(format!("{row_id} carries no commitment")))
}

fn status(expected: &str, actual: &str) -> &'static str {
    if expected == actual {
        "matched"
    } else {
        "mismatched"
    }
}

/// An archive is verified by its bytes.
///
/// # Errors
/// Fails if the archive cannot be read.
pub fn verify_archive(archive: &Path, expected: &str) -> Result<Value> {
    let actual = format!("sha256:{}", sha256_file(archive)?);
    Ok(json!({
        "expected": expected,
        "actual": actual,
        "status": status(expected, &actual),
        "method": "sha256 of the archive bytes",
    }))
}

/// A directory is verified by re-creating the committed tar stream. When
/// `tar` is unavailable the check is reported as unverified, never as matched.
#[must_use]
pub fn verify_directory(dir: &Path, expected: &str) -> Value {
    let method = format!(
        "tar {} -cf - -C <dir> . | sha256",
        COMMITMENT_TAR_FLAGS.join(" ")
    );
    let output = command("tar")
        .args(COMMITMENT_TAR_FLAGS)
        .args(["-cf", "-", "-C"])
        .arg(dir)
        .arg(".")
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let actual = format!("sha256:{}", sha256_hex(&out.stdout));
            json!({
                "expected": expected,
                "actual": actual,
                "status": status(expected, &actual),
                "method": method,
            })
        }
        _ => json!({
            "expected": expected,
            "status": "unverified",
            "method": method,
            "reason": "tar is unavailable or failed",
        }),
    }
}

/// An archive member must stay inside the extraction directory.
#[must_use]
pub fn safe_member(name: &str) -> bool {
    let path = Path::new(name);
    !path.is_absolute()
        && !path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
}

/// Whether every line of a `tar -tv` listing is a regular file (`-`) or a
/// directory (`d`).
#[must_use]
pub fn only_files_and_directories(verbose_listing: &str) -> bool {
    verbose_listing
        .lines()
        .filter(|l| !l.is_empty())
        .all(|l| matches!(l.as_bytes().first(), Some(b'-' | b'd')))
}

/// Extracts an archive into `dest` after checking every member name. The
/// offending name of an unsafe member is not reported: it is private.
///
/// # Errors
/// Fails on an unsafe member, a missing `tar`, or a failed extraction.
pub fn extract(archive: &Path, dest: &Path) -> Result<()> {
    let listing = command("tar")
        .arg("-tf")
        .arg(archive)
        .output()
        .context(|| "listing the archive".to_owned())?;
    if !listing.status.success() {
        return Err(Error::new("the archive could not be listed"));
    }
    let names = String::from_utf8_lossy(&listing.stdout);
    if names.lines().any(|n| !safe_member(n)) {
        return Err(Error::new(
            "an archive member would escape the extraction directory",
        ));
    }
    // A safe name is not enough: a symlink or hard-link member can redirect a
    // later member outside the directory. Only regular files and directories
    // are accepted, read from the type column of the verbose listing
    // (review finding on 5ef607c, CHG-004.6.1).
    let verbose = command("tar")
        .arg("-tvf")
        .arg(archive)
        .output()
        .context(|| "listing the archive's member types".to_owned())?;
    if !verbose.status.success() {
        return Err(Error::new("the archive could not be listed"));
    }
    if !only_files_and_directories(&String::from_utf8_lossy(&verbose.stdout)) {
        return Err(Error::new(
            "an archive member is a link or special file; only files and directories are accepted",
        ));
    }
    std::fs::create_dir_all(dest)
        .context(|| "creating the private extraction directory".to_owned())?;
    let extracted = command("tar")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .output()
        .context(|| "extracting the archive".to_owned())?;
    if !extracted.status.success() {
        return Err(Error::new("the archive could not be extracted"));
    }
    Ok(())
}

/// Ready workspaces under `source`: directories holding both `Cargo.toml`
/// and `rha-crates.toml`, in sorted order, never through a symbolic link.
///
/// # Errors
/// Fails on an unreadable directory or a linked entry point.
pub fn workspaces(source: &Path) -> Result<Vec<PathBuf>> {
    fn visit(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        let manifest = dir.join("Cargo.toml");
        let rules = dir.join("rha-crates.toml");
        if manifest.is_file() && rules.is_file() {
            if manifest.is_symlink() || rules.is_symlink() {
                return Err(Error::new("a workspace entry point is a symbolic link"));
            }
            out.push(dir.to_path_buf());
        }
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .context(|| "reading a case directory".to_owned())?
            .collect::<std::io::Result<Vec<_>>>()
            .context(|| "reading a case directory entry".to_owned())?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name();
            if name == "target" || name == ".git" || name == ".jj" {
                continue;
            }
            let path = entry.path();
            if path.is_symlink() || !path.is_dir() {
                continue;
            }
            visit(&path, out)?;
        }
        Ok(())
    }
    let mut out = Vec::new();
    visit(source, &mut out)?;
    Ok(out)
}

/// Markdown sites for `--kind check`: every immediate subdirectory, sorted,
/// not hidden, not a link.
///
/// # Errors
/// On an unreadable directory.
pub fn sites(source: &Path) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(source)
        .context(|| "reading the sites directory".to_owned())?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| !p.is_symlink() && p.is_dir())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('.'))
        })
        .collect();
    out.sort();
    Ok(out)
}

/// One site's bounded `rhawiki check` observation: exit status and witness
/// counts by kind, never the witnesses themselves.
fn observe_check(root: &Path, id: &str, site: &Path) -> Result<(Value, Vec<u8>, Vec<u8>)> {
    let output = command("cargo")
        .args([
            "run", "-q", "-p", "app-cli", "--", "check", "--format", "json", "--root",
        ])
        .arg(site)
        .current_dir(root)
        .output()
        .context(|| "running rhawiki check".to_owned())?;
    let exit = output.status.code();
    let report: Option<Value> = serde_json::from_slice(&output.stdout).ok();
    let mut row = json!({"case_id": id, "exit_status": exit, "graded": false});
    let outcome = match (
        exit,
        report.as_ref().and_then(|r| r["witnesses"].as_array()),
    ) {
        (Some(code @ (0 | 1)), Some(ws)) if code == i32::from(!ws.is_empty()) => {
            row["witnesses"] = json!(ws.len());
            row["counts"] = report.as_ref().map_or(Value::Null, |r| r["counts"].clone());
            "observed"
        }
        (Some(2), _) => "config_error",
        _ => "tool_error",
    };
    row["outcome"] = json!(outcome);
    Ok((row, output.stdout, output.stderr))
}

/// One case's bounded observation, with the raw output returned for the
/// caller to keep privately.
fn observe(root: &Path, id: &str, workspace: &Path) -> Result<(Value, Vec<u8>, Vec<u8>)> {
    let output = command("cargo")
        .args(["xtask", "architecture", "--manifest-path"])
        .arg(workspace.join("Cargo.toml"))
        .arg("--rules")
        .arg(workspace.join("rha-crates.toml"))
        .args(["--format", "json"])
        .current_dir(root)
        .output()
        .context(|| "running the checker".to_owned())?;
    let exit = output.status.code();
    let report: Option<Value> = serde_json::from_slice(&output.stdout).ok();
    let summary = report
        .as_ref()
        .and_then(|r| r.get("summary"))
        .cloned()
        .unwrap_or(Value::Null);
    let mut row = json!({"case_id": id, "exit_status": exit, "graded": false});
    let outcome = match exit {
        Some(2) => "config_error",
        Some(0 | 1) => match (summary["errors"].as_u64(), summary["warnings"].as_u64()) {
            (Some(errors), Some(warnings)) if exit == Some(i32::from(errors > 0)) => {
                row["errors"] = json!(errors);
                row["warnings"] = json!(warnings);
                "observed"
            }
            _ => "tool_error",
        },
        _ => "tool_error",
    };
    row["outcome"] = json!(outcome);
    Ok((row, output.stdout, output.stderr))
}

fn default_private_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir)
        .join("rha/held-out")
}

#[cfg(unix)]
fn restrict(dir: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn restrict(_dir: &Path) {}

fn not_run(reason: &str, check: Option<Value>) -> Value {
    let mut summary = json!({"outcome": "not_run", "graded": false, "reason": reason});
    if let Some(check) = check {
        summary["commitment"] = check;
    }
    summary
}

/// Runs the private cases and returns the bounded summary with its exit
/// code: 0 when every case was observed, 2 when nothing ran, 3 when a case
/// could not be observed.
///
/// # Errors
/// Fails on ledger, archive, file-system or checker-spawn failures.
pub fn execute(root: &Path, args: &HeldOutArgs) -> Result<(Value, u8)> {
    let row = commitment_row(&args.kind);
    // Private cases need their commitment; public controls only report it.
    let expected = match commitment(root, row) {
        Ok(c) => c,
        Err(_) if args.purpose == "held_out" => {
            return Ok((
                not_run(
                    &format!("{row} carries no commitment yet, so nothing can be verified"),
                    None,
                ),
                2,
            ));
        }
        Err(_) => format!("none: {row} carries no commitment"),
    };
    let now = UtcTime::now();
    let run_dir = args
        .private_dir
        .clone()
        .unwrap_or_else(default_private_dir)
        .join(format!("run-{}", now.compact()));
    std::fs::create_dir_all(&run_dir)
        .context(|| "creating the private run directory".to_owned())?;
    restrict(&run_dir);
    let (source, check) = match (&args.archive, &args.cases) {
        (Some(archive), _) => {
            let check = verify_archive(archive, &expected)?;
            if check["status"] != "matched" && args.purpose == "held_out" {
                return Ok((
                    not_run(
                        "the archive does not match the DP-1.1b commitment",
                        Some(check),
                    ),
                    2,
                ));
            }
            let inputs = run_dir.join("inputs");
            extract(archive, &inputs)?;
            (inputs, check)
        }
        (None, Some(dir)) => {
            let check = verify_directory(dir, &expected);
            // Unverified is not matched: without the check, altered cases
            // would be observed and reported as if they were the committed
            // ones (review finding on 5ef607c, CHG-004.6.1).
            if check["status"] != "matched" && args.purpose == "held_out" {
                return Ok((
                    not_run(
                        "the directory does not verify against the DP-1.1b commitment",
                        Some(check),
                    ),
                    2,
                ));
            }
            (dir.clone(), check)
        }
        (None, None) => return Ok((not_run("no --archive or --cases was given", None), 2)),
    };
    let check_kind = args.kind == "check";
    let cases = if check_kind {
        sites(&source)?
    } else {
        workspaces(&source)?
    };
    if cases.is_empty() {
        return Ok((
            not_run(
                if check_kind {
                    "no site directory was found"
                } else {
                    "no ready workspace with Cargo.toml and rha-crates.toml was found"
                },
                Some(check),
            ),
            2,
        ));
    }
    let report_path = root.join(crate::architecture::REPORT_PATH);
    let original = std::fs::read(&report_path).ok();
    let mut rows = Vec::new();
    let mut map = serde_json::Map::new();
    for (index, workspace) in cases.iter().enumerate() {
        let id = format!("H{:03}", index + 1);
        let (row, stdout, stderr) = if check_kind {
            observe_check(root, &id, workspace)?
        } else {
            observe(root, &id, workspace)?
        };
        std::fs::write(run_dir.join(format!("{id}.stdout")), stdout)
            .context(|| "keeping a raw report privately".to_owned())?;
        std::fs::write(run_dir.join(format!("{id}.stderr")), stderr)
            .context(|| "keeping a raw report privately".to_owned())?;
        let relative = workspace.strip_prefix(&source).unwrap_or(workspace);
        map.insert(id, json!(relative.display().to_string()));
        rows.push(row);
    }
    match original {
        Some(bytes) => {
            let _ = std::fs::write(&report_path, bytes);
        }
        None => {
            let _ = std::fs::remove_file(&report_path);
        }
    }
    std::fs::write(
        run_dir.join("private-case-map.json"),
        serde_json::to_vec_pretty(&Value::Object(map))
            .context(|| "serializing the private case map".to_owned())?,
    )
    .context(|| "writing the private case map".to_owned())?;
    let all_observed = rows.iter().all(|row| row["outcome"] == "observed");
    let identity = git_identity(root);
    let summary = json!({
        "schema_version": 1,
        "kind": "held_out_observation",
        "purpose": args.purpose,
        "decision": "DP-1.1b, as amended in CHG-004.6",
        "graded": false,
        "reason": "Observations of the accepted checker. Whether each matches Kennedy's expectation is his comparison, recorded in the acceptance record.",
        "observed_at": now.rfc3339(),
        "checker": {
            "git_rev": identity.as_ref().map_or("unknown", |i| i.revision.as_str()),
            "git_dirty": identity.as_ref().map(|i| i.dirty),
        },
        "commitment": check,
        "cases": rows,
        "outcome": if all_observed { "observed" } else { "failed_execution" },
        "private_reports": run_dir.display().to_string(),
    });
    std::fs::write(
        run_dir.join("summary.json"),
        serde_json::to_vec_pretty(&summary).context(|| "serializing the summary".to_owned())?,
    )
    .context(|| "writing the private summary".to_owned())?;
    Ok((summary, if all_observed { 0 } else { 3 }))
}

/// The command: prints the bounded summary and returns its exit code.
///
/// # Errors
/// See [`execute`].
pub fn run(root: &Path, args: &HeldOutArgs) -> Result<u8> {
    let (mut summary, code) = execute(root, args)?;
    // The report directory is written to summary.json inside it, not printed:
    // what is printed is what an agent reads (CHG-004.6.1).
    if let Some(object) = summary.as_object_mut() {
        object.remove("private_reports");
    }
    let mut text =
        serde_json::to_string_pretty(&summary).context(|| "serializing the summary".to_owned())?;
    text.push('\n');
    print!("{text}");
    Ok(code)
}
