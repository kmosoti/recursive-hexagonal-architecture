use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use super::registered_package;

pub fn root(package: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../xtask/tests/corpus/transclusion")
        .join(package)
}

pub fn load_at(root: &Path, package: &str) -> Result<Vec<Value>, String> {
    let mut files = BTreeSet::new();
    scan_tree(root, root, None, &mut files)?;

    let mut payloads = BTreeMap::new();
    for relative in files {
        let bytes = fs::read(root.join(&relative))
            .map_err(|error| format!("{}: {error}", root.join(&relative).display()))?;
        payloads.insert(relative, bytes);
    }

    registered_package::load_from_payloads(package, &payloads)
}

fn scan_tree(
    root: &Path,
    path: &Path,
    relative: Option<&str>,
    files: &mut BTreeSet<String>,
) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "symlink found before payload read: {}",
            path.display()
        ));
    }

    if metadata.is_dir() {
        if let Some(relative) = relative
            && !safe_relative_path(relative)
        {
            return Err(format!("unsafe directory path: {relative}"));
        }
        let entries = fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
            let child = entry.path();
            let child_relative = child
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or_else(|| format!("non-UTF-8 path: {}", child.display()))?
                .replace(std::path::MAIN_SEPARATOR, "/");
            scan_tree(root, &child, Some(&child_relative), files)?;
        }
    } else if metadata.is_file() {
        let relative = relative.ok_or_else(|| format!("file root: {}", path.display()))?;
        if !safe_relative_path(relative) {
            return Err(format!("unsafe file path: {relative}"));
        }
        files.insert(relative.to_owned());
    } else {
        return Err(format!("non-file special entry: {}", path.display()));
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(value).is_relative()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
