//! Capture the identity of the compiled reporter.
//!
//! This is deliberately a standard-library-only build script. The identity is
//! tied to the source checkout that built `xtask`, so inspecting another
//! workspace at runtime cannot rewrite the report's tool fields.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest_dir.parent().unwrap_or(&manifest_dir);

    println!("cargo:rerun-if-changed={}", manifest_dir.display());
    for path in [
        root.join("Cargo.toml"),
        root.join("xtask/Cargo.toml"),
        root.join("Cargo.lock"),
        root.join("rust-toolchain.toml"),
        root.join(".cargo/config.toml"),
        root.join("clippy.toml"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for path in git_watch_paths(root) {
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let revision = git_stdout(root, &["rev-parse", "--verify", "HEAD"])
        .filter(|value| is_full_revision(value))
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = match git_status(root) {
        Some(output) if output.is_empty() => "false",
        Some(_) => "true",
        None => "unknown",
    };
    println!("cargo:rustc-env=RHA_TOOL_GIT_REV={revision}");
    println!("cargo:rustc-env=RHA_TOOL_GIT_DIRTY={dirty}");
}

fn git_command(root: &Path, args: &[&str]) -> Option<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
}

fn git_stdout(root: &Path, args: &[&str]) -> Option<String> {
    let output = git_command(root, args)?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn git_status(root: &Path) -> Option<String> {
    let output = git_command(
        root,
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--",
            "xtask",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            "clippy.toml",
        ],
    )?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn is_full_revision(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Return the small set of git state files whose changes can alter the build
/// identity. In a linked worktree `.git` is a file, and the actual HEAD,
/// symbolic ref, packed refs, and index live in paths resolved by git itself.
fn git_watch_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let git_dir = git_path(root, &["rev-parse", "--git-dir"]);
    let git_common_dir = git_path(root, &["rev-parse", "--git-common-dir"]);
    add_exact_watch(
        &mut paths,
        git_path(root, &["rev-parse", "--git-path", "HEAD"]),
    );
    if let Some(reference) = git_stdout(root, &["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git_path(root, &["rev-parse", "--git-path", &reference])
    {
        let mut stops = Vec::new();
        if let Some(path) = git_dir.as_deref() {
            stops.push(path);
        }
        if let Some(path) = git_common_dir.as_deref() {
            stops.push(path);
        }
        add_watch_path(&mut paths, Some(path), &stops);
    }
    for name in ["packed-refs", "index"] {
        add_exact_watch(
            &mut paths,
            git_path(root, &["rev-parse", "--git-path", name]),
        );
    }
    paths
}

fn add_exact_watch(paths: &mut Vec<PathBuf>, path: Option<PathBuf>) {
    if let Some(path) = path.filter(|path| path.exists()) {
        paths.push(path);
    }
}

/// Watch an existing git state file, or the nearest existing parent when a
/// loose ref has already been packed. The git directory itself is excluded:
/// watching it would make every unrelated repository update rebuild xtask.
fn add_watch_path(paths: &mut Vec<PathBuf>, path: Option<PathBuf>, stops: &[&Path]) {
    let Some(mut candidate) = path else { return };
    loop {
        if candidate.exists() {
            if !stops.contains(&candidate.as_path()) {
                paths.push(candidate);
            }
            return;
        }
        let Some(parent) = candidate.parent() else {
            return;
        };
        if stops.contains(&parent) {
            return;
        }
        candidate = parent.to_path_buf();
    }
}

fn git_path(root: &Path, args: &[&str]) -> Option<PathBuf> {
    let path = PathBuf::from(git_stdout(root, args)?);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(root.join(path))
    }
}
