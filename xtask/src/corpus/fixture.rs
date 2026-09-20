//! Deterministic materialization of declared cases. No expected result is
//! consulted here; Cargo sees the dependency spellings in the registration.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

use super::{Case, Dep, DepKind, Generation, Manifest};
use crate::error::{Context as _, Error, Result};

fn write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<()> {
    let parent = path.parent().ok_or_else(|| Error::new("missing parent"))?;
    std::fs::create_dir_all(parent).context(|| format!("creating {}", parent.display()))?;
    std::fs::write(path, bytes).context(|| format!("writing {}", path.display()))
}

fn toml_file(path: &Path, value: &Value) -> Result<()> {
    write(
        path,
        toml::to_string_pretty(value).context(|| "serializing fixture TOML".to_owned())?,
    )
}

fn name(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::new(format!("unsafe fixture name: {value}")));
    }
    Ok(())
}

fn normalize(path: &Path) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if !out.pop() {
                    return Err(Error::new("fixture path escapes root"));
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    Ok(out)
}

fn relative(from: &Path, to: &Path) -> PathBuf {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let shared = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in shared..a.len() {
        out.push("..");
    }
    for component in &b[shared..] {
        out.push(component.as_os_str());
    }
    out
}

fn dependency(dep: &Dep, from: &Path, locations: &BTreeMap<String, PathBuf>) -> Result<Value> {
    name(&dep.name)?;
    if let Some(rename) = &dep.rename {
        name(rename)?;
    }
    let mut value = json!({});
    match dep.source.as_deref().unwrap_or("path") {
        "registry" => {
            value["version"] = json!(
                dep.version
                    .as_deref()
                    .ok_or_else(|| Error::new("registry dependency needs a version"))?
            );
        }
        "path" => {
            let target = locations
                .get(&dep.name)
                .ok_or_else(|| Error::new(format!("unknown dependency {}", dep.name)))?;
            value["path"] = json!(relative(from, target));
        }
        source => {
            return Err(Error::new(format!(
                "unsupported dependency source {source}"
            )));
        }
    }
    if dep.rename.is_some() {
        value["package"] = json!(dep.name);
    }
    Ok(value)
}

fn dependencies(
    deps: &[Dep],
    from: &Path,
    locations: &BTreeMap<String, PathBuf>,
    inherit: bool,
) -> Result<Value> {
    let mut table = json!({});
    for dep in deps {
        let key = dep.rename.as_deref().unwrap_or(&dep.name);
        let mut value = if dep.inherit && inherit {
            json!({"workspace": true})
        } else {
            dependency(dep, from, locations)?
        };
        if dep.optional {
            value["optional"] = json!(true);
        }
        let section = match dep.kind {
            DepKind::Normal => "dependencies",
            DepKind::Dev => "dev-dependencies",
            DepKind::Build => "build-dependencies",
        };
        let parent = if let Some(target) = &dep.target {
            &mut table["target"][target]
        } else {
            &mut table
        };
        if !parent[section][key].is_null() {
            return Err(Error::new(format!("duplicate dependency {key}")));
        }
        parent[section][key] = value;
    }
    Ok(table)
}

fn source(deps: &[Dep], core: bool, body: Option<&str>) -> String {
    let mut text = String::new();
    if core {
        text.push_str("#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]\n");
    }
    for dep in deps {
        if dep.kind == DepKind::Build {
            continue;
        }
        if let Some(target) = &dep.target {
            text.push_str(&format!("#[{target}]\n"));
        }
        if dep.kind == DepKind::Dev {
            text.push_str("#[cfg(test)]\n");
        }
        if dep.optional {
            text.push_str(&format!(
                "#[cfg(feature = {:?})]\n",
                dep.rename.as_deref().unwrap_or(&dep.name)
            ));
        }
        let ident = dep.rename.as_deref().unwrap_or(&dep.name).replace('-', "_");
        text.push_str(&format!("#[allow(unused_imports)] use {ident} as _;\n"));
    }
    if let Some(body) = body {
        text.push_str(body);
        text.push('\n');
    }
    text
}

/// Generate a case below a fresh, absolute directory. Outside crates are
/// siblings of `workspace`, contained within this case's directory.
///
/// # Errors
/// Rejects unsupported declarations, path escapes, duplicate locations and I/O failures.
pub fn generate(case: &Case, manifest: &Manifest, base: &Path, template: &[u8]) -> Result<PathBuf> {
    if case.generation != Generation::Declared || !base.is_absolute() {
        return Err(Error::new(
            "crate generation requires a declared case and absolute directory",
        ));
    }
    name(&case.id)?;
    std::fs::create_dir(base).context(|| format!("creating fresh fixture {}", base.display()))?;
    let workspace = base.join("workspace");
    let mut locations = BTreeMap::new();
    for krate in &case.crates {
        name(&krate.name)?;
        if locations
            .insert(krate.name.clone(), workspace.join(&krate.name))
            .is_some()
        {
            return Err(Error::new("duplicate fixture crate"));
        }
    }
    for krate in &case.outside_crates {
        name(&krate.name)?;
        let path = normalize(&workspace.join(&krate.at))?;
        if !path.starts_with(base)
            || path.starts_with(&workspace)
            || path == base
            || locations
                .values()
                .any(|p| p.starts_with(&path) || path.starts_with(p))
        {
            return Err(Error::new(format!(
                "outside crate {} must be isolated beside workspace",
                krate.name
            )));
        }
        if locations.insert(krate.name.clone(), path).is_some() {
            return Err(Error::new("duplicate fixture crate"));
        }
    }
    let mut root = json!({"workspace": {"members": case.crates.iter().map(|c| &c.name).collect::<Vec<_>>(), "resolver": "3"}});
    for krate in &case.crates {
        for dep in krate.deps.iter().filter(|d| d.inherit) {
            let key = dep.rename.as_deref().unwrap_or(&dep.name);
            let value = dependency(dep, &workspace, &locations)?;
            let previous = &root["workspace"]["dependencies"][key];
            if !previous.is_null() && previous != &value {
                return Err(Error::new("conflicting inherited dependency"));
            }
            root["workspace"]["dependencies"][key] = value;
        }
    }
    toml_file(&workspace.join("Cargo.toml"), &root)?;
    for krate in &case.crates {
        let dir = &locations[&krate.name];
        let mut config = dependencies(&krate.deps, dir, &locations, true)?;
        config["package"] =
            json!({"name": krate.name, "version": "0.0.0", "edition": "2024", "publish": false});
        if let Some(role) = &krate.role {
            config["package"]["metadata"]["rha"]["role"] = json!(role);
        }
        if !krate.implements.is_empty() {
            config["package"]["metadata"]["rha"]["implements"] = json!(krate.implements);
        }
        toml_file(&dir.join("Cargo.toml"), &config)?;
        let core = krate.role.as_deref() == Some("core");
        write(
            &dir.join("src/lib.rs"),
            source(&krate.deps, core, krate.body.as_deref()),
        )?;
        if core {
            write(&dir.join("clippy.toml"), template)?;
        }
        if krate.build_script {
            write(&dir.join("build.rs"), "fn main() {}\n")?;
        }
    }
    for krate in &case.outside_crates {
        let dir = &locations[&krate.name];
        let mut config = dependencies(&krate.deps, dir, &locations, false)?;
        config["package"] =
            json!({"name": krate.name, "version": "0.0.0", "edition": "2024", "publish": false});
        config["workspace"] = json!({});
        toml_file(&dir.join("Cargo.toml"), &config)?;
        write(&dir.join("src/lib.rs"), source(&krate.deps, false, None))?;
    }
    let d = &manifest.rules_default;
    let r = &case.rules;
    let rules = json!({
        "schema_version": 1,
        "classification": {
            "adapter_prefix": r.adapter_prefix.as_ref().unwrap_or(&d.adapter_prefix),
            "app_prefix": r.app_prefix.as_ref().unwrap_or(&d.app_prefix),
            "tools": r.tools.as_ref().unwrap_or(&d.tools),
            "harness": r.harness.as_ref().unwrap_or(&d.harness),
        },
        "core": {
            "allow": r.core_allow.as_ref().unwrap_or(&d.core_allow),
            "dev_allow": r.core_dev_allow.as_ref().unwrap_or(&d.core_dev_allow),
            "allow_build_scripts": r.core_allow_build_scripts.unwrap_or(d.core_allow_build_scripts),
        },
        "adapters": {"require_port_owner_dependency": r.adapters_require_port_owner_dependency.unwrap_or(d.adapters_require_port_owner_dependency)},
        "transitive": {"enabled": r.transitive_enabled.unwrap_or(d.transitive_enabled)},
        "forbidden": r.forbidden_edges.as_ref().unwrap_or(&d.forbidden_edges),
    });
    toml_file(&workspace.join("rha-crates.toml"), &rules)?;
    Ok(workspace)
}

/// Sorted input digests captured before Cargo can add lockfiles or output.
///
/// # Errors
/// Returns directory and file read failures.
pub fn digests(base: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(base: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
        for entry in std::fs::read_dir(dir).context(|| "listing fixture inputs".to_owned())? {
            let entry = entry.context(|| "reading fixture entry".to_owned())?;
            let path = entry.path();
            if path.is_dir() {
                visit(base, &path, out)?;
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .map_err(|e| Error::new(e.to_string()))?
                        .display()
                        .to_string(),
                    crate::util::sha256_file(&path)?,
                );
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(base, base, &mut out)?;
    Ok(out)
}

/// Committed fixture tree. Outside crates are siblings of their case workspace.
pub const COMMITTED_ROOT: &str = "xtask/tests/corpus/crate";

#[must_use]
pub fn case_path(case: &Case) -> PathBuf {
    let group = match case.expected {
        super::Expected::Detect => "violations",
        super::Expected::NoAlarm => "legitimate",
        super::Expected::ExpectedMiss => "expected_miss",
        super::Expected::Reference => "reference",
    };
    PathBuf::from(group).join(&case.id)
}

/// Derive all committed bytes from registration; never consult stored fixtures.
///
/// # Errors
/// Returns invalid declaration, conflicting generated path or I/O errors.
pub fn expected_tree(root: &Path, manifest: &Manifest) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let scratch = root.join("target/rha").join(format!(
        "fixture-generation-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&scratch).context(|| "creating generation scratch".to_owned())?;
    let result = (|| {
        let template = std::fs::read(root.join(crate::clippy_template::TEMPLATE_PATH))
            .context(|| "reading core template".to_owned())?;
        let mut files = BTreeMap::new();
        for case in manifest
            .cases
            .iter()
            .filter(|case| case.level == super::Level::Crate)
        {
            let container = scratch.join(&case.id);
            generate(case, manifest, &container, &template)?;
            let relative = case_path(case);
            for file in digests(&container)?.keys() {
                let source = Path::new(file);
                let destination = if let Ok(member) = source.strip_prefix("workspace") {
                    relative.join(member)
                } else {
                    relative
                        .parent()
                        .ok_or_else(|| Error::new("case group absent"))?
                        .join(source)
                };
                let bytes = std::fs::read(container.join(source))
                    .context(|| "reading generated input".to_owned())?;
                if files
                    .get(&destination)
                    .is_some_and(|previous| previous != &bytes)
                {
                    return Err(Error::new(format!(
                        "conflicting generated file {}",
                        destination.display()
                    )));
                }
                files.insert(destination, bytes);
            }
        }
        Ok(files)
    })();
    std::fs::remove_dir_all(&scratch).context(|| "removing generation scratch".to_owned())?;
    result
}

fn committed_files(base: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).context(|| "listing committed fixture inputs".to_owned())? {
        let entry = entry.context(|| "reading committed fixture entry".to_owned())?;
        let name = entry.file_name();
        if name == "target" || name == "Cargo.lock" {
            continue;
        }
        let kind = entry
            .file_type()
            .context(|| "reading fixture file type".to_owned())?;
        if kind.is_symlink() {
            return Err(Error::new("fixture inputs must not be symbolic links"));
        }
        let path = entry.path();
        if kind.is_dir() {
            committed_files(base, &path, files)?;
        } else {
            files.insert(
                path.strip_prefix(base)
                    .map_err(|e| Error::new(e.to_string()))?
                    .to_path_buf(),
                std::fs::read(&path).context(|| "reading committed fixture".to_owned())?,
            );
        }
    }
    Ok(())
}

/// Exact added/removed/changed file comparison, excluding only Cargo outputs.
///
/// # Errors
/// Returns unreadable or unsafe fixture inputs.
pub fn drift(base: &Path, expected: &BTreeMap<PathBuf, Vec<u8>>) -> Result<Vec<PathBuf>> {
    let mut actual = BTreeMap::new();
    committed_files(base, base, &mut actual)?;
    let keys: std::collections::BTreeSet<_> = expected.keys().chain(actual.keys()).collect();
    Ok(keys
        .into_iter()
        .filter(|path| expected.get(*path) != actual.get(*path))
        .cloned()
        .collect())
}

/// Explicit fixture-generation command. `--check` never repairs drift.
///
/// # Errors
/// Returns manifest/generation or file-writing failures.
pub fn sync(root: &Path, check: bool) -> Result<u8> {
    let text = std::fs::read_to_string(root.join(super::MANIFEST_PATH))
        .context(|| "reading corpus registration".to_owned())?;
    let manifest = Manifest::parse(&text).context(|| "parsing corpus registration".to_owned())?;
    if manifest.schema_version != 1 || !manifest.defects().is_empty() {
        return Err(Error::new("invalid corpus registration"));
    }
    let expected = expected_tree(root, &manifest)?;
    let base = root.join(COMMITTED_ROOT);
    if !check {
        for (path, bytes) in &expected {
            write(&base.join(path), bytes)?;
        }
    }
    let differences = drift(&base, &expected)?;
    for path in &differences {
        eprintln!("fixture drift: {}", path.display());
    }
    println!(
        "{} generated fixture inputs; {} differences",
        expected.len(),
        differences.len()
    );
    Ok(u8::from(!differences.is_empty()))
}
