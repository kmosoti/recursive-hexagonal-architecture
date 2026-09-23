//! `cargo xtask scope --task <id>`: the scope and layout guard (plan §2.2 M4).
//!
//! Every path the change touches, compared with the merge base and including
//! uncommitted and untracked files, must match one of the task record's
//! `scope_globs`, must sit in the repository layout of plan §4, and, if it is
//! a protected surface (`.rha/policy.toml [surface] protected`), must also
//! match the task's `protected_scope`. Lesson L4: an executor improvised a
//! `scripts/` directory, journals under `docs/`, and an edit to a protected
//! template, and each was caught by a person, late.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::{Context as _, Error, Result};
use crate::util::command_stdout;

/// Top-level entries of the repository layout (plan §4).
pub const TOP_LEVEL: [&str; 22] = [
    ".cargo",
    ".config",
    ".github",
    ".gitignore",
    ".rha",
    "AGENTS.md",
    "CONTRIBUTING.md",
    "Cargo.lock",
    "Cargo.toml",
    "README.md",
    "_typos.toml",
    "clippy.toml",
    "crates",
    "deny.toml",
    "docs",
    "evidence",
    "experiments",
    "rha-baseline.json",
    "rha-crates.toml",
    "rust-toolchain.toml",
    "tools",
    "xtask",
];

/// Entries of `docs/` (plan §3.4, and the plan and proposal files).
pub const DOCS: [&str; 17] = [
    "adr",
    "architecture",
    "changes",
    "conformance",
    "enforcement-map.md",
    "evidence",
    "index.md",
    "maturity.md",
    "observations",
    "orchestration-log.md",
    "plan",
    "proposals",
    "spec",
    "tasks",
    "threat-model.md",
    "toolchain.md",
    "benchmarks",
];

/// Entries of `.rha/` (plan §4.1).
pub const RHA: [&str; 8] = [
    "acceptances",
    "architecture.toml",
    "assumptions.toml",
    "decisions.toml",
    "exceptions.log",
    "policy.toml",
    "schemas",
    "tasks",
];

/// Matches a `/`-separated path against a glob: `**` is any number of whole
/// segments, including none; `*` is any run of characters inside a segment.
#[must_use]
pub fn glob(pattern: &str, path: &str) -> bool {
    fn segment(pattern: &[u8], text: &[u8]) -> bool {
        match pattern.split_first() {
            None => text.is_empty(),
            Some((b'*', rest)) => (0..=text.len()).any(|i| segment(rest, &text[i..])),
            Some((c, rest)) => text.first() == Some(c) && segment(rest, &text[1..]),
        }
    }
    fn walk(pattern: &[&str], path: &[&str]) -> bool {
        match pattern.split_first() {
            None => path.is_empty(),
            Some((&"**", rest)) => (0..=path.len()).any(|i| walk(rest, &path[i..])),
            Some((p, rest)) => path
                .split_first()
                .is_some_and(|(s, tail)| segment(p.as_bytes(), s.as_bytes()) && walk(rest, tail)),
        }
    }
    let p: Vec<&str> = pattern.split('/').collect();
    let s: Vec<&str> = path.split('/').collect();
    walk(&p, &s)
}

/// Why a path is outside the layout, or `None` when it is inside.
#[must_use]
pub fn layout_problem(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    let top = parts.next().unwrap_or_default();
    if !TOP_LEVEL.contains(&top) {
        return Some(format!(
            "`{top}` is not a top-level entry of the plan §4 layout"
        ));
    }
    let second = parts.next();
    match (top, second) {
        ("docs", Some(entry)) if !DOCS.contains(&entry) => Some(format!(
            "`docs/{entry}` is not part of the plan §3.4 docs layout"
        )),
        (".rha", Some(entry)) if !RHA.contains(&entry) => Some(format!(
            "`.rha/{entry}` is not part of the plan §4.1 record layout"
        )),
        _ => None,
    }
}

/// The findings for one set of changed paths. Pure, so it is tested without
/// git.
#[must_use]
pub fn check(
    changed: &BTreeSet<String>,
    scope_globs: &[String],
    protected_scope: &[String],
    protected: &[String],
) -> Vec<Value> {
    let mut findings = Vec::new();
    for path in changed {
        if let Some(problem) = layout_problem(path) {
            findings.push(json!({"path": path, "rule": "scope.layout", "message": problem}));
        }
        if !scope_globs.iter().any(|g| glob(g, path)) {
            findings.push(json!({"path": path, "rule": "scope.outside_task", "message": "matches none of the task record's scope_globs"}));
        }
        if protected.iter().any(|g| glob(g, path)) && !protected_scope.iter().any(|g| glob(g, path))
        {
            findings.push(json!({"path": path, "rule": "scope.protected_unapproved", "message": "a protected surface that the task record's protected_scope does not name"}));
        }
    }
    findings
}

fn strings(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// The task record with this id.
///
/// # Errors
/// Fails if no task record has the id, or it cannot be read.
pub fn task(root: &Path, id: &str) -> Result<toml::Value> {
    let dir = root.join(".rha/tasks");
    for entry in std::fs::read_dir(&dir).context(|| "reading .rha/tasks".to_owned())? {
        let path = entry
            .context(|| "reading a task record entry".to_owned())?
            .path();
        if path.extension().is_none_or(|e| e != "toml") {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).context(|| format!("reading {}", path.display()))?;
        let value: toml::Value =
            toml::from_str(&text).context(|| format!("parsing {}", path.display()))?;
        if value.get("id").and_then(toml::Value::as_str) == Some(id) {
            return Ok(value);
        }
    }
    Err(Error::new(format!(
        "no task record in .rha/tasks has id {id}"
    )))
}

/// Paths changed since the merge base with `base`, committed or not. Git's
/// path quoting is off: by default it prints `café.md` as a quoted string of
/// octal escapes,
/// which matched no glob (found by the markdown corpus in P-A stage 1).
///
/// # Errors
/// Fails if git cannot answer.
pub fn changed(root: &Path, base: &str) -> Result<BTreeSet<String>> {
    let merge_base = command_stdout(root, &["git", "merge-base", "HEAD", base])?;
    let mut out = BTreeSet::new();
    for line in command_stdout(
        root,
        &[
            "git",
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-only",
            // Both endpoints of a rename: a moved protected file must be
            // seen at its source too (review finding 9).
            "--no-renames",
            &merge_base,
        ],
    )?
    .lines()
    {
        out.insert(line.to_owned());
    }
    for line in command_stdout(
        root,
        &[
            "git",
            "-c",
            "core.quotePath=false",
            "ls-files",
            "--others",
            "--exclude-standard",
        ],
    )?
    .lines()
    {
        out.insert(line.to_owned());
    }
    out.remove("");
    Ok(out)
}

/// The command. Exit 0 in scope, 1 with findings, 2 when the task record has
/// no `scope_globs` (a guard that cannot be evaluated is not a pass).
///
/// # Errors
/// Fails on unreadable records or a git failure.
pub fn run(root: &Path, id: &str, base: &str) -> Result<u8> {
    let record = task(root, id)?;
    let scope_globs = strings(record.get("scope_globs"));
    if scope_globs.is_empty() {
        eprintln!("xtask scope: task {id} declares no scope_globs, so its scope cannot be checked");
        return Ok(2);
    }
    let protected_scope = strings(record.get("protected_scope"));
    let policy = crate::policy::load(root)?;
    let changed = changed(root, base)?;
    let findings = check(
        &changed,
        &scope_globs,
        &protected_scope,
        &policy.policy.surface.protected,
    );
    for f in &findings {
        println!(
            "{}[{}]: {}",
            f["path"].as_str().unwrap_or_default(),
            f["rule"].as_str().unwrap_or_default(),
            f["message"].as_str().unwrap_or_default()
        );
    }
    println!(
        "scope: {} changed path(s), {} finding(s), task {id}, base {base}",
        changed.len(),
        findings.len()
    );
    Ok(u8::from(!findings.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn globs_match_segments_and_double_stars() {
        assert!(glob("docs/**", "docs/changes/CHG-005.md"));
        assert!(glob(
            "crates/*/rha-modules.toml",
            "crates/site/rha-modules.toml"
        ));
        assert!(!glob(
            "crates/*/rha-modules.toml",
            "crates/site/src/rha-modules.toml"
        ));
        assert!(glob("xtask/**", "xtask"), "** matches zero segments");
        assert!(glob("evidence/CHG-*/**", "evidence/CHG-005/a.json"));
        assert!(!glob("docs/*.md", "docs/changes/a.md"));
        assert!(glob("Cargo.lock", "Cargo.lock"));
        // Matching is byte-wise, so a non-ASCII segment cannot split a
        // character (PR 16 thread: refuted, kept as a regression test).
        assert!(glob("docs/*.md", "docs/café.md"));
        assert!(glob("docs/c*é.md", "docs/café.md"));
        assert!(!glob("docs/*.rs", "docs/café.md"));
    }

    #[test]
    fn the_improvisations_of_lesson_l4_are_caught() {
        let scope = vec!["**".to_owned()];
        let protected = vec![".github/**".to_owned(), "rha-crates.toml".to_owned()];
        let found = check(
            &set(&[
                "scripts/run_held_out.py",
                "docs/unapproved-journal.md",
                ".github/pull_request_template.md",
            ]),
            &scope,
            &[],
            &protected,
        );
        let rules: Vec<(&str, &str)> = found
            .iter()
            .map(|f| {
                (
                    f["path"].as_str().unwrap_or_default(),
                    f["rule"].as_str().unwrap_or_default(),
                )
            })
            .collect();
        assert!(rules.contains(&("scripts/run_held_out.py", "scope.layout")));
        assert!(rules.contains(&("docs/unapproved-journal.md", "scope.layout")));
        assert!(layout_problem("docs/orchestration-log.md").is_none());
        assert!(rules.contains(&(
            ".github/pull_request_template.md",
            "scope.protected_unapproved"
        )));
    }

    #[test]
    fn a_path_outside_the_task_scope_is_reported_and_an_approved_protected_one_is_not() {
        let scope = vec!["xtask/**".to_owned(), "rha-crates.toml".to_owned()];
        let protected = vec!["rha-crates.toml".to_owned()];
        let approved = vec!["rha-crates.toml".to_owned()];
        assert!(
            check(
                &set(&["xtask/src/scope.rs", "rha-crates.toml"]),
                &scope,
                &approved,
                &protected
            )
            .is_empty()
        );
        let found = check(
            &set(&["crates/site/src/lib.rs"]),
            &scope,
            &approved,
            &protected,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["rule"], "scope.outside_task");
    }

    #[test]
    fn the_current_repository_fits_its_own_layout() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("root");
        let tracked = command_stdout(root, &["git", "-c", "core.quotePath=false", "ls-files"])
            .expect("git ls-files");
        let outside: Vec<&str> = tracked
            .lines()
            .filter(|p| layout_problem(p).is_some())
            .collect();
        assert!(outside.is_empty(), "{outside:?}");
    }
}
