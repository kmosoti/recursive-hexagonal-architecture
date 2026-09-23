//! `rha-verifier corpus <dir> <out-dir>`: decides every fixture in `dir`,
//! compares each decision with its registered `expected`, and writes an
//! H5 evidence record (plan W16) naming the checked-out revision.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The registration's tree digest: sha256 over the sorted lines
/// `"<path> <sha256>\n"` of every file below `root` except the registration
/// and the generator prompt, as `xtask/tests/corpus_trust.rs` computes it.
fn tree_digest(root: &Path) -> Result<String, String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<(), String> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                walk(&path, base, out)?;
            } else if name != "registration.toml" && name != "GENERATOR-PROMPT.md" {
                let rel = path
                    .strip_prefix(base)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                out.push(format!("{rel} {}\n", hex(&Sha256::digest(&bytes))));
            }
        }
        Ok(())
    }
    let mut lines = Vec::new();
    walk(root, root, &mut lines)?;
    lines.sort();
    Ok(hex(&Sha256::digest(lines.concat().as_bytes())))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Refuses a directory that is not the corpus its `registration.toml`
/// registered: no registration, a changed tree, or no fixtures at all
/// (PR 17, automated review thread). Returns the registered digest.
fn registered(dir: &Path) -> Result<String, String> {
    let path = dir.join("registration.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("{}: {e}; refusing an unregistered corpus", path.display()))?;
    let registration: toml::Table =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let expected = registration
        .get("tree_sha256")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{}: no tree_sha256", path.display()))?;
    let actual = tree_digest(dir)?;
    if actual != expected {
        return Err(format!(
            "{}: tree_sha256 is {actual}, registered {expected}; refusing a changed corpus",
            dir.display()
        ));
    }
    Ok(expected.to_owned())
}

fn run(dir: &Path, out_dir: &Path) -> Result<bool, String> {
    let tree_sha256 = registered(dir)?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut cases = Vec::new();
    for path in &files {
        let fixture: Value =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        let decision = rha_verifier::evaluate(&fixture);
        let expected = &fixture["expected"];
        let wrong: Vec<&String> = expected
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, _)| k)
            .filter(|k| decision[k.as_str()] != expected[k.as_str()])
            .collect();
        cases.push(json!({"id": fixture["id"], "row": fixture["row"], "passed": wrong.is_empty(), "keys_wrong": wrong, "merge_allowed": decision["merge_allowed"]}));
    }
    if cases.is_empty() {
        return Err(format!(
            "{}: no fixtures; refusing a vacuous pass",
            dir.display()
        ));
    }
    let passed = cases.iter().filter(|c| c["passed"] == true).count();
    let revision = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    let dirty = git(&["status", "--porcelain", "--untracked-files=all"]).map(|s| !s.is_empty());
    let record = json!({
        "schema_version": 1, "kind": "h5_conformance", "evidence_class": "local", "advisory": true,
        "subject": {"revision": revision, "dirty": dirty},
        "corpus": {"path": dir.display().to_string(), "tree_sha256": tree_sha256},
        "summary": {"fixtures": cases.len(), "decided_as_registered": passed, "outcome": if passed == cases.len() { "passed" } else { "failed" }},
        "not_run": ["the isolation exercise: no protected runner exists", "Authentic on real records: no trusted producer keys (DP-4.1)"],
        "cases": cases,
    });
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let short: String = revision.chars().take(12).collect();
    let path = out_dir.join(format!(
        "{short}{}.json",
        if dirty == Some(true) { "-dirty" } else { "" }
    ));
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "H5 conformance: {passed}/{} fixtures decided as registered; evidence {}",
        record["summary"]["fixtures"],
        path.display()
    );
    Ok(passed
        == record["summary"]["fixtures"]
            .as_u64()
            .map_or(0, |n| usize::try_from(n).unwrap_or(0)))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, cmd, dir, out] if cmd == "corpus" => match run(Path::new(dir), Path::new(out)) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(e) => {
                eprintln!("rha-verifier: {e}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("usage: rha-verifier corpus <fixture-dir> <evidence-dir>");
            ExitCode::from(2)
        }
    }
}
