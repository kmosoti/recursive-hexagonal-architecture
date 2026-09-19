//! Artifact identity of the tested input state (spec §11.3).
//!
//! A clean checkout is identified by its commit and tree. A dirty working tree
//! is additionally identified by a digest of the tracked diff, the digests of
//! untracked inputs, and `snapshot_tree`: the tree object of the whole working
//! state, written through a temporary index so the real index is untouched.

use std::path::Path;

use serde::Serialize;

use crate::error::{Context as _, Error, Result};
use crate::util::{command, command_stdout, sha256_file, sha256_hex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Subject {
    pub revision: String,
    pub tree: String,
    pub snapshot_tree: String,
    pub branch: String,
    pub dirty: bool,
    pub tracked_diff_sha256: Option<String>,
    pub untracked_inputs: Vec<UntrackedInput>,
    pub git_object_format: String,
    pub digest_algorithm: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UntrackedInput {
    pub path: String,
    pub sha256: String,
}

/// Captures the identity of the working tree at `root`. `scratch` holds the
/// temporary index.
///
/// # Errors
/// Fails if `root` is not a git work tree with at least one commit.
pub fn capture(root: &Path, scratch: &Path) -> Result<Subject> {
    let git = |args: &[&str]| -> Result<String> {
        let mut argv = vec!["git"];
        argv.extend_from_slice(args);
        command_stdout(root, &argv)
    };
    let revision = git(&["rev-parse", "HEAD"])?;
    let tree = git(&["rev-parse", "HEAD^{tree}"])?;
    let git_object_format = git(&["rev-parse", "--show-object-format"])?;
    let branch = match std::env::var("GITHUB_HEAD_REF") {
        Ok(head) if !head.is_empty() => head,
        _ => git(&["rev-parse", "--abbrev-ref", "HEAD"])?,
    };

    let diff = command_bytes(root, &["git", "diff", "HEAD", "--binary"])?;
    let tracked_diff_sha256 = (!diff.is_empty()).then(|| sha256_hex(&diff));

    let listing = command_bytes(
        root,
        &["git", "ls-files", "--others", "--exclude-standard", "-z"],
    )?;
    let mut untracked_inputs = Vec::new();
    for raw in listing.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = String::from_utf8_lossy(raw).into_owned();
        let sha256 = sha256_file(&root.join(&path))?;
        untracked_inputs.push(UntrackedInput { path, sha256 });
    }

    let snapshot_tree = snapshot_tree(root, scratch)?;
    Ok(Subject {
        dirty: tracked_diff_sha256.is_some() || !untracked_inputs.is_empty(),
        revision,
        tree,
        snapshot_tree,
        branch,
        tracked_diff_sha256,
        untracked_inputs,
        git_object_format,
        digest_algorithm: "sha256",
    })
}

/// The tree of the full working state (tracked and untracked, not ignored).
fn snapshot_tree(root: &Path, scratch: &Path) -> Result<String> {
    std::fs::create_dir_all(scratch).context(|| format!("creating {}", scratch.display()))?;
    let index = scratch.join("snapshot.index");
    let _ = std::fs::remove_file(&index);
    for args in [&["read-tree", "HEAD"][..], &["add", "-A"][..]] {
        let status = command("git")
            .args(args)
            .env("GIT_INDEX_FILE", &index)
            .current_dir(root)
            .status()
            .context(|| format!("running git {}", args.join(" ")))?;
        if !status.success() {
            return Err(Error::new(format!(
                "git {} with a temporary index failed",
                args.join(" ")
            )));
        }
    }
    let output = command("git")
        .arg("write-tree")
        .env("GIT_INDEX_FILE", &index)
        .current_dir(root)
        .output()
        .context(|| "running git write-tree".to_owned())?;
    let _ = std::fs::remove_file(&index);
    if !output.status.success() {
        return Err(Error::new("git write-tree with a temporary index failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn command_bytes(root: &Path, argv: &[&str]) -> Result<Vec<u8>> {
    let (program, rest) = argv
        .split_first()
        .ok_or_else(|| Error::new("empty command"))?;
    let output = command(program)
        .args(rest)
        .current_dir(root)
        .output()
        .context(|| format!("running `{}`", argv.join(" ")))?;
    if !output.status.success() {
        return Err(Error::new(format!(
            "`{}` exited with {}",
            argv.join(" "),
            output.status
        )));
    }
    Ok(output.stdout)
}

/// The base revision `b` and how it was determined. CI passes it in
/// `RHA_BASE_SHA`; locally it is the merge base with `origin/main`.
#[must_use]
pub fn base_revision(root: &Path) -> (String, String) {
    if let Ok(sha) = std::env::var("RHA_BASE_SHA")
        && !sha.is_empty()
    {
        return (sha, "env RHA_BASE_SHA".to_owned());
    }
    match command_stdout(root, &["git", "merge-base", "HEAD", "origin/main"]) {
        Ok(sha) => (sha, "git merge-base HEAD origin/main".to_owned()),
        Err(_) => (
            "unknown".to_owned(),
            "no RHA_BASE_SHA and no merge base with origin/main".to_owned(),
        ),
    }
}

/// The digest of `path` as committed at `revision`: `Ok(None)` when the
/// commit exists but has no such file.
///
/// # Errors
/// Fails when the commit object is not available locally.
pub fn digest_at(root: &Path, revision: &str, path: &str) -> Result<Option<String>> {
    command_stdout(
        root,
        &["git", "cat-file", "-e", &format!("{revision}^{{commit}}")],
    )
    .map_err(|_| Error::new(format!("commit {revision} is not available locally")))?;
    let spec = format!("{revision}:{path}");
    match command_bytes(root, &["git", "show", &spec]) {
        Ok(bytes) => Ok(Some(format!("sha256:{}", sha256_hex(&bytes)))),
        Err(_) => Ok(None),
    }
}
