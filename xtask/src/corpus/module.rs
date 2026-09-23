//! Module-level H4 corpus execution and evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::Output;

use serde_json::{Value, json};

use super::{Case, Expected, Level, MANIFEST_PATH, Manifest, grade, runner};
use crate::cli::CorpusArgs;
use crate::error::{Context as _, Error, Result};
use crate::util::{UtcTime, command, command_stdout, sha256_file, sha256_hex};

const MODULE_ROOT: &str = "xtask/tests/corpus/module";
const HEADLINE_MANIFEST_SHA256: &str =
    "07059dd444a7023552026ea3fa3051b71290b030c0d960f5ca3d1f1509196e03";
const REFERENCE_SHA256: &str = "dd4ce1ce37e619b054860c9428f48b0cdcc3e4672d166cbd7803c41ed85d36fa";
const CONTENT_SHA256: &str = "2091eb8990f95c9736e0eecf2ff877ddcdee75df51aa28ed74c916bfbb797f8a";

const AUTHORED_IDS: [&str; 25] = [
    "M01", "M02", "M03", "M04", "M05", "M06", "M07", "M08", "M09", "M10", "M11", "M12", "M13",
    "M14", "M15", "M16", "M17", "M18", "M19", "M20", "M21", "L-M02", "EM-M01", "EM-M02", "EM-M03",
];

const ALL_IDS: [&str; 27] = [
    "M01", "M02", "M03", "M04", "M05", "M06", "M07", "M08", "M09", "M10", "M11", "M12", "M13",
    "M14", "M15", "M16", "M17", "M18", "M19", "M20", "M21", "L-M01", "L-M02", "EM-M01", "EM-M02",
    "EM-M03", "X-M01",
];

const MODULE_CELLS: [&str; 4] = ["d2", "law2-d3", "law3-d1", "law6-b3"];

struct Process {
    argv: Vec<String>,
    output: Output,
}

impl Process {
    fn exit_status(&self) -> Option<i32> {
        self.output.status.code()
    }

    fn json(&self) -> Value {
        json!({
            "argv": self.argv,
            "exit_status": self.exit_status(),
            "stdout": String::from_utf8_lossy(&self.output.stdout),
            "stderr": String::from_utf8_lossy(&self.output.stderr),
        })
    }
}

fn run_process(root: &Path, argv: &[String], target_dir: Option<&Path>) -> Result<Process> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| Error::new("empty subprocess argv"))?;
    let mut command = command(program);
    command.args(args).current_dir(root);
    if let Some(target_dir) = target_dir {
        command.env("CARGO_TARGET_DIR", target_dir);
    }
    let output = command
        .output()
        .context(|| format!("spawning `{}`", argv.join(" ")))?;
    Ok(Process {
        argv: argv.to_vec(),
        output,
    })
}

fn numeric_exit(process: &Process, label: &str) -> Result<i32> {
    process
        .exit_status()
        .ok_or_else(|| Error::new(format!("{label} terminated without an exit status")))
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && Path::new(path).is_relative()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn walk_payloads(
    root: &Path,
    directory: &Path,
    payloads: &mut BTreeMap<String, String>,
    bytes: &mut u64,
) -> Result<()> {
    let mut entries = std::fs::read_dir(directory)
        .context(|| format!("reading module inventory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .context(|| format!("reading module inventory {}", directory.display()))?;
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .context(|| format!("reading metadata for {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::new(format!(
                "module inventory contains a symlink: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                return Err(Error::new(format!(
                    "generated target directory is inside module inventory: {}",
                    path.display()
                )));
            }
            walk_payloads(root, &path, payloads, bytes)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(Error::new(format!(
                "module inventory contains a non-file: {}",
                path.display()
            )));
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|error| Error::new(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "registration.toml" || relative == "SHA256SUMS" {
            continue;
        }
        if relative.split('/').any(|part| part == "target") {
            return Err(Error::new(format!(
                "generated target input is inside module inventory: {relative}"
            )));
        }
        *bytes += metadata.len();
        payloads.insert(relative, sha256_file(&path)?);
    }
    Ok(())
}

/// Verify the frozen module inventory and return its actual payload hashes.
pub fn verify_inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    let sums_path = root.join("SHA256SUMS");
    let registration_path = root.join("registration.toml");
    let sums_bytes =
        std::fs::read(&sums_path).context(|| "reading module SHA256SUMS".to_owned())?;
    let registration_text = std::fs::read_to_string(&registration_path)
        .context(|| "reading module registration".to_owned())?;
    let registration: toml::Value =
        toml::from_str(&registration_text).context(|| "parsing module registration".to_owned())?;

    let content_sha = registration["content_sha256"]
        .as_str()
        .ok_or_else(|| Error::new("module registration lacks content_sha256"))?;
    if content_sha != CONTENT_SHA256 || sha256_hex(&sums_bytes) != content_sha {
        return Err(Error::new(
            "module SHA256SUMS does not match the registered content_sha256",
        ));
    }

    let text =
        std::str::from_utf8(&sums_bytes).context(|| "decoding module SHA256SUMS".to_owned())?;
    if !sums_bytes.ends_with(b"\n") {
        return Err(Error::new("module SHA256SUMS must end with LF"));
    }

    let mut listed = BTreeMap::new();
    let mut previous: Option<String> = None;
    for line in text.lines() {
        if line.len() < 66 || line.as_bytes().get(64..66) != Some(b"  ") {
            return Err(Error::new(format!(
                "malformed module SHA256SUMS line: {line:?}"
            )));
        }
        let digest = &line[..64];
        let relative = &line[66..];
        if !lower_sha256(digest) || !safe_relative(relative) {
            return Err(Error::new(format!(
                "unsafe module SHA256SUMS line: {line:?}"
            )));
        }
        if previous
            .as_deref()
            .is_some_and(|old| old.split('/').cmp(relative.split('/')).is_ge())
        {
            return Err(Error::new(format!(
                "module SHA256SUMS is not strictly sorted at {relative}"
            )));
        }
        previous = Some(relative.to_owned());
        if listed
            .insert(relative.to_owned(), digest.to_owned())
            .is_some()
        {
            return Err(Error::new(format!(
                "duplicate module SHA256SUMS path: {relative}"
            )));
        }

        let path = root.join(relative);
        let metadata = std::fs::symlink_metadata(&path)
            .context(|| format!("reading listed module payload {relative}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Error::new(format!(
                "listed module payload is not a durable file: {relative}"
            )));
        }
        if sha256_file(&path)? != digest {
            return Err(Error::new(format!(
                "module payload digest mismatch: {relative}"
            )));
        }
    }

    let mut actual = BTreeMap::new();
    let mut payload_bytes = 0;
    walk_payloads(root, root, &mut actual, &mut payload_bytes)?;
    if actual != listed {
        return Err(Error::new(format!(
            "module inventory differs from SHA256SUMS: listed {}, actual {}",
            listed.len(),
            actual.len()
        )));
    }
    if registration["content_files"].as_integer() != Some(listed.len() as i64) {
        return Err(Error::new(
            "module registration content_files does not match the inventory",
        ));
    }
    if registration["content_bytes"].as_integer() != Some(payload_bytes as i64) {
        return Err(Error::new(
            "module registration content_bytes does not match the inventory",
        ));
    }
    Ok(actual)
}

fn registration_count(registration: &toml::Value, key: &str, expected: i64) -> Result<()> {
    if registration["counts"][key].as_integer() != Some(expected) {
        return Err(Error::new(format!(
            "module registration counts.{key} is not the frozen value {expected}"
        )));
    }
    Ok(())
}

pub fn verify_registration_contract(
    root: &Path,
    payloads: &BTreeMap<String, String>,
) -> Result<()> {
    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(root.join("registration.toml"))
            .context(|| "reading module registration".to_owned())?,
    )
    .context(|| "parsing module registration".to_owned())?;

    let manifest_path = root
        .parent()
        .ok_or_else(|| Error::new("module corpus has no parent"))?
        .join("manifest.toml");
    if sha256_file(&manifest_path)? != HEADLINE_MANIFEST_SHA256
        || registration["headline_manifest_sha256"].as_str() != Some(HEADLINE_MANIFEST_SHA256)
    {
        return Err(Error::new(
            "headline manifest does not match its frozen registration digest",
        ));
    }
    if sha256_file(&root.join("reference.py"))? != REFERENCE_SHA256
        || registration["reference_sha256"].as_str() != Some(REFERENCE_SHA256)
    {
        return Err(Error::new(
            "module reference implementation does not match its registered digest",
        ));
    }
    if registration["headline_author_sha256"].as_str()
        != Some(
            sha256_file(&root.join("headline.py"))
                .context(|| "hashing headline generator".to_owned())?
                .as_str(),
        )
    {
        return Err(Error::new(
            "headline generator does not match its registered digest",
        ));
    }

    let section_digest = |prefix: &str| {
        let mut entries = payloads
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .collect::<Vec<_>>();
        entries.sort_by(|(a, _), (b, _)| a.split('/').cmp(b.split('/')));
        let bytes = entries
            .into_iter()
            .map(|(path, digest)| format!("{digest}  {path}\n"))
            .collect::<String>();
        sha256_hex(bytes.as_bytes())
    };
    if registration["random_content_sha256"].as_str() != Some(section_digest("random/").as_str())
        || registration["headline_content_sha256"].as_str()
            != Some(section_digest("headline/").as_str())
    {
        return Err(Error::new(
            "module section content digest differs from registration",
        ));
    }

    for (key, value) in [
        ("headline_edges", 41),
        ("headline_fixtures", 25),
        ("headline_include_fragments", 1),
        ("headline_limitations", 4),
        ("headline_rust_files", 27),
        ("optional_cycles", 64),
        ("optional_subsumed_undeclared_edges", 64),
        ("optional_top_level_findings", 192),
        ("optional_top_level_undeclared_edges", 64),
        ("optional_total_undeclared_edge_facts", 128),
        ("preserved_original_artifacts", 672),
        ("random_cases", 256),
        ("random_edges", 5229),
        ("random_heuristic_edges", 116),
        ("random_limitations", 0),
        ("random_rust_files", 1228),
        ("random_source_map_files", 1740),
        ("random_test_edges", 512),
        ("rule_exhaustive_graph_checks", 4096),
        ("rule_negative_mutation_checks", 6),
        ("rule_targeted_checks", 7),
        ("unique_flat_component_topologies", 117),
        ("rustc_legality_checks", 512),
        ("cargo_headline_checks", 25),
        ("reference_hand_tests", 14),
        ("legal_random_cases", 64),
        ("violating_random_cases", 192),
    ] {
        registration_count(&registration, key, value)?;
    }
    Ok(())
}

fn manifest_module_cases(root: &Path) -> Result<(Manifest, Vec<Case>)> {
    let text = std::fs::read_to_string(root.join(MANIFEST_PATH))
        .context(|| "reading public corpus manifest".to_owned())?;
    let manifest =
        Manifest::parse(&text).context(|| "parsing public corpus manifest".to_owned())?;
    let defects = manifest.defects();
    if manifest.schema_version != 1 || !defects.is_empty() {
        return Err(Error::new(format!("invalid corpus manifest: {defects:?}")));
    }

    let cases = manifest
        .cases
        .iter()
        .filter(|case| case.level == Level::Module)
        .cloned()
        .collect::<Vec<_>>();
    let actual = cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = ALL_IDS.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(Error::new(
            "manifest module case ids differ from the frozen set",
        ));
    }
    if cases.len() != ALL_IDS.len() {
        return Err(Error::new(
            "manifest does not contain exactly 27 module cases",
        ));
    }
    Ok((manifest, cases))
}

fn headline_fixtures(root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let headline = root.join("headline");
    let mut actual = BTreeMap::new();
    for entry in
        std::fs::read_dir(&headline).context(|| format!("reading {}", headline.display()))?
    {
        let entry = entry.context(|| "reading headline fixture entry".to_owned())?;
        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).context(|| format!("reading {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::new(format!(
                "headline fixture is a symlink: {}",
                path.display()
            )));
        }
        if !metadata.is_dir() {
            return Err(Error::new(format!(
                "headline fixture entry is not a directory: {}",
                path.display()
            )));
        }
        let id = entry
            .file_name()
            .to_str()
            .ok_or_else(|| Error::new("headline fixture id is not UTF-8"))?
            .to_owned();
        if actual.insert(id.clone(), path).is_some() {
            return Err(Error::new(format!("duplicate headline fixture {id}")));
        }
    }
    let expected = AUTHORED_IDS.iter().copied().collect::<BTreeSet<_>>();
    if actual.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err(Error::new(
            "headline fixture ids differ from the 25 authored manifest cases",
        ));
    }
    for (id, path) in &actual {
        for required in ["Cargo.toml", "rha-modules.toml"] {
            if !path.join(required).is_file() || path.join(required).is_symlink() {
                return Err(Error::new(format!("{id} lacks durable {required}")));
            }
        }
    }
    Ok(actual)
}

fn package_identity(fixture: &Path) -> Result<(String, String)> {
    let text = std::fs::read_to_string(fixture.join("Cargo.toml"))
        .context(|| format!("reading {}", fixture.join("Cargo.toml").display()))?;
    let cargo: toml::Value = toml::from_str(&text)
        .context(|| format!("parsing {}", fixture.join("Cargo.toml").display()))?;
    let package = cargo
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::new("fixture Cargo.toml lacks [package]"))?;
    let name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::new("fixture package lacks a name"))?
        .to_owned();
    let edition = package
        .get("edition")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Error::new("fixture package lacks an edition"))?
        .to_owned();
    Ok((name, edition))
}

fn fixture_digests(root: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(base: &Path, directory: &Path, output: &mut BTreeMap<String, String>) -> Result<()> {
        let mut entries = std::fs::read_dir(directory)
            .context(|| format!("reading fixture inputs {}", directory.display()))?
            .collect::<std::io::Result<Vec<_>>>()
            .context(|| format!("reading fixture inputs {}", directory.display()))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)
                .context(|| format!("reading {}", path.display()))?;
            if metadata.file_type().is_symlink() {
                return Err(Error::new(format!(
                    "fixture input is a symlink: {}",
                    path.display()
                )));
            }
            if metadata.is_dir() {
                if path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                visit(base, &path, output)?;
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(base)
                    .map_err(|error| Error::new(error.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                output.insert(relative, sha256_file(&path)?);
            }
        }
        Ok(())
    }

    let mut output = BTreeMap::new();
    visit(root, root, &mut output)?;
    Ok(output)
}

fn module_repository_root() -> PathBuf {
    if Path::new(MODULE_ROOT).is_dir() {
        return PathBuf::from(".");
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn normalized_edges(value: &Value, label: &str) -> Result<Vec<(String, String, bool, String)>> {
    let entries = value
        .as_array()
        .ok_or_else(|| Error::new(format!("{label} is not an array")))?;
    let mut normalized = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let source = entry["source"]
            .as_str()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks string source")))?;
        let target = entry["target"]
            .as_str()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks string target")))?;
        let test_only = entry["test_only"]
            .as_bool()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks boolean test_only")))?;
        let extraction = entry["extraction"]
            .as_str()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks string extraction")))?;
        normalized.push((
            source.to_owned(),
            target.to_owned(),
            test_only,
            extraction.to_owned(),
        ));
    }
    normalized.sort();
    Ok(normalized)
}

fn normalized_limitations(value: &Value, label: &str) -> Result<Vec<(String, String, String)>> {
    let entries = value
        .as_array()
        .ok_or_else(|| Error::new(format!("{label} is not an array")))?;
    let mut normalized = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let source = entry["source"]
            .as_str()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks string source")))?;
        let code = entry["code"]
            .as_str()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks string code")))?;
        let detail = entry["detail"]
            .as_str()
            .ok_or_else(|| Error::new(format!("{label}[{index}] lacks string detail")))?;
        normalized.push((source.to_owned(), code.to_owned(), detail.to_owned()));
    }
    normalized.sort();
    Ok(normalized)
}

fn is_runtime_source_approximation(value: &Value, package: &str) -> bool {
    value["code"] == "source_approximation"
        && value["crate"] == package
        && value["package"] == package
        && value["source"].is_null()
        && value["detail"]
            == "module extraction is source-based; heuristic paths and unexpanded macros or include! remain limitations"
}

/// Validate the registered reference facts for an authored headline fixture.
pub fn validate_headline_reference(fixture: &Path, report: &Value) -> Result<()> {
    let reference_path = fixture.join("reference.json");
    let metadata = std::fs::symlink_metadata(&reference_path)
        .context(|| format!("reading {}", reference_path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::new(format!(
            "headline reference is not a durable file: {}",
            reference_path.display()
        )));
    }
    let reference: Value = serde_json::from_slice(
        &std::fs::read(&reference_path)
            .context(|| format!("reading {}", reference_path.display()))?,
    )
    .context(|| format!("parsing {}", reference_path.display()))?;
    if reference["schema_version"] != 1 {
        return Err(Error::new(
            "headline reference has an unsupported schema version",
        ));
    }

    let expected_edges = normalized_edges(&reference["edges"], "headline reference edges")?;
    let observed_edges = normalized_edges(&report["module_edges"], "observed module edges")?;
    if observed_edges != expected_edges {
        return Err(Error::new(
            "observed module edges differ from the registered headline reference",
        ));
    }

    let (package, _) = package_identity(fixture)?;
    let observed_limitations = report["limitations"]
        .as_array()
        .ok_or_else(|| Error::new("observed limitations is not an array"))?
        .iter()
        .filter(|limitation| !is_runtime_source_approximation(limitation, &package))
        .cloned()
        .collect::<Vec<_>>();
    let observed_limitations = normalized_limitations(
        &Value::Array(observed_limitations),
        "observed registered limitations",
    )?;
    let expected_limitations =
        normalized_limitations(&reference["limitations"], "headline reference limitations")?;
    if observed_limitations != expected_limitations {
        return Err(Error::new(
            "observed limitations differ from the registered headline reference",
        ));
    }
    Ok(())
}

fn finding_failure(mut value: grade::Grade, reason: impl Into<String>) -> grade::Grade {
    value.passed = false;
    value.detected = false;
    value.documented_miss = false;
    value.reasons.push(reason.into());
    value
}

fn module_grade(
    case: &Case,
    exit: Option<i32>,
    report: &Value,
    validation: Result<()>,
) -> grade::Grade {
    let mut value = grade::architecture(case, exit, report);
    if let Err(error) = validation {
        value = finding_failure(value, error.to_string());
    }
    if case.id == "EM-M03" {
        let visible = report["module_edges"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|edge| {
                edge_owner(&edge["source"]) == Some("constraints")
                    && edge_owner(&edge["target"]) == Some("ordering")
            });
        let implicit_model = report["module_edges"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|edge| {
                edge_owner(&edge["source"]) == Some("constraints")
                    && edge_owner(&edge["target"]) == Some("model")
            });
        if !visible {
            value = finding_failure(
                value,
                "EM-M03 did not retain the visible constraints-to-ordering edge",
            );
        }
        if implicit_model {
            value = finding_failure(
                value,
                "EM-M03 invented an implicit constraints-to-model edge",
            );
        }
    }
    value.passed = value.reasons.is_empty();
    value
}

fn edge_owner(value: &Value) -> Option<&str> {
    let path = value.as_str()?;
    let mut parts = path.split("::");
    let first = parts.next()?;
    if first == "crate" || first.starts_with("fixture_") {
        parts.next()
    } else {
        Some(first)
    }
}

fn path_is_inside(path: &str, root: &Path) -> bool {
    let candidate = Path::new(path);
    candidate.starts_with(root) && candidate.is_file()
}

/// Validate the complete module-only report against the invoked fixture.
pub fn validate_report(_root: &Path, fixture: &Path, report: &Value, tool: &Value) -> Result<()> {
    let (package, edition) = package_identity(fixture)?;
    let source = fixture.join("src/lib.rs");
    let manifest = fixture.join("Cargo.toml");
    let rules = fixture.join("rha-modules.toml");
    let fixture_root = std::fs::canonicalize(fixture)
        .context(|| format!("canonicalizing {}", fixture.display()))?;
    let source = std::fs::canonicalize(&source)
        .context(|| format!("canonicalizing {}", source.display()))?;
    let manifest = std::fs::canonicalize(&manifest)
        .context(|| format!("canonicalizing {}", manifest.display()))?;
    let rules =
        std::fs::canonicalize(&rules).context(|| format!("canonicalizing {}", rules.display()))?;

    if report["schema_version"] != 1
        || report["tool"] != *tool
        || report["subject"]["mode"] != "module_only"
        || report["subject"]["workspace_root"] != json!(fixture_root.display().to_string())
        || report["subject"]["manifest_path"] != json!(manifest.display().to_string())
        || report["subject"]["metadata_mode"] != "no_deps"
        || report["subject"]["source"] != json!(source.display().to_string())
        || report["subject"]["crate_name"] != package.replace('-', "_")
        || report["subject"]["edition"] != edition
        || report["subject"]["rules_path"] != json!(rules.display().to_string())
        || report["subject"]["source_sha256"] != format!("sha256:{}", sha256_file(&source)?)
        || report["subject"]["manifest_sha256"] != format!("sha256:{}", sha256_file(&manifest)?)
        || report["subject"]["rules_digest"] != json!(format!("sha256:{}", sha256_file(&rules)?))
    {
        return Err(Error::new(
            "module-only report producer, subject, source or rules identity differs from the invocation",
        ));
    }

    // Every .rs input in the frozen authored fixtures is part of their module
    // tree. Check the independent committed file population, not a list
    // supplied by the checker under test.
    let expected_sources = fixture_digests(&fixture_root)?
        .into_iter()
        .filter(|(path, _)| Path::new(path).extension().is_some_and(|ext| ext == "rs"))
        .map(|(path, digest)| (fixture_root.join(path).display().to_string(), digest))
        .collect::<BTreeMap<_, _>>();
    if report["subject"]["source_files_sha256"] != json!(expected_sources) {
        return Err(Error::new(
            "module-only source input population or digests differ from the frozen fixture",
        ));
    }
    for field in [
        "findings",
        "test_edges",
        "module_edges",
        "limitations",
        "module_checks",
    ] {
        if !report[field].is_array() {
            return Err(Error::new(format!(
                "module-only report lacks {field} array"
            )));
        }
    }
    let checks = report["module_checks"]
        .as_array()
        .ok_or_else(|| Error::new("module-only report lacks module_checks"))?;
    if checks.len() != 1 {
        return Err(Error::new(
            "module-only report must contain exactly one module check",
        ));
    }
    for check in checks {
        if check["source_files_sha256"] != json!(expected_sources)
            || check["outcome"] != report["summary"]["outcome"]
            || check["required"] != true
            || !matches!(check["outcome"].as_str(), Some("passed" | "failed"))
            || check["crate"] != package
            || check["rules_path"] != json!(rules.display().to_string())
            || check["rules_digest"] != json!(format!("sha256:{}", sha256_file(&rules)?))
        {
            return Err(Error::new(
                "module-only report contains a missing or unevaluated module check",
            ));
        }
        if check["source"].as_str().is_none_or(|path| {
            !path_is_inside(path, &fixture_root) || path != source.display().to_string()
        }) {
            return Err(Error::new(
                "module-only module check does not identify the invoked source",
            ));
        }
        if check["extraction"]["counts"]["modules"]
            .as_u64()
            .is_none_or(|count| count == 0)
        {
            return Err(Error::new(
                "module-only report has no positive extracted module count",
            ));
        }
    }

    for finding in report["findings"].as_array().into_iter().flatten() {
        if finding["manifest_path"] != json!(manifest.display().to_string()) {
            return Err(Error::new(
                "module finding does not identify the invoked fixture manifest",
            ));
        }
        if finding["crate"].is_string() && finding["crate"] != package.replace('-', "_") {
            return Err(Error::new("module finding names the wrong crate"));
        }
        if finding["package"].is_string() && finding["package"] != package {
            return Err(Error::new("module finding names the wrong package"));
        }
        let witness = finding["witness"]
            .as_object()
            .ok_or_else(|| Error::new("module finding lacks a witness object"))?;
        if witness.get("rules_file").and_then(Value::as_str)
            != Some(rules.display().to_string().as_str())
        {
            return Err(Error::new(
                "module finding witness does not identify the invoked rules file",
            ));
        }
        for key in ["source_file", "target_file"] {
            if let Some(path) = witness.get(key).and_then(Value::as_str)
                && !path_is_inside(path, &fixture_root)
            {
                return Err(Error::new(format!(
                    "module finding witness {key} escapes the fixture"
                )));
            }
        }
    }
    Ok(())
}

fn validate_full_site_report(root: &Path, report: &Value, tool: &Value) -> Result<()> {
    let manifest = std::fs::canonicalize(root.join("crates/site/Cargo.toml"))
        .context(|| "canonicalizing site Cargo.toml".to_owned())?;
    let source = std::fs::canonicalize(root.join("crates/site/src/lib.rs"))
        .context(|| "canonicalizing site src/lib.rs".to_owned())?;
    let rules = std::fs::canonicalize(root.join("crates/site/rha-modules.toml"))
        .context(|| "canonicalizing site rha-modules.toml".to_owned())?;
    if report["schema_version"] != 1
        || report["tool"] != *tool
        || report["subject"]["manifest_path"] != json!(manifest.display().to_string())
    {
        return Err(Error::new(
            "full site architecture report has incorrect producer or manifest identity",
        ));
    }
    let check = report["module_checks"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|check| check["crate"] == "site")
        .ok_or_else(|| Error::new("full site report lacks the site module check"))?;
    if check["required"] != true
        || check["outcome"] != "passed"
        || check["source"] != json!(source.display().to_string())
        || check["rules_path"] != json!(rules.display().to_string())
        || check["rules_digest"] != json!(format!("sha256:{}", sha256_file(&rules)?))
        || check["extraction"]["counts"]["modules"]
            .as_u64()
            .is_none_or(|count| count == 0)
    {
        return Err(Error::new(
            "full site report does not contain a real passing module check",
        ));
    }
    Ok(())
}

fn add_runs(mut report: Value, runs: &[Value]) -> Value {
    report["runs"] = json!(runs);
    report
}

fn case_record(
    case: &Case,
    fixture_path: &str,
    process: &Process,
    report: Value,
    grade: grade::Grade,
    runs: &[Value],
) -> Value {
    json!({
        "id": case.id,
        "cells": case.cells,
        "expected": expectation(case.expected),
        "detector": "xtask architecture",
        "fixture_path": fixture_path,
        "argv": process.argv,
        "exit_status": process.exit_status(),
        "stdout": String::from_utf8_lossy(&process.output.stdout),
        "stderr": String::from_utf8_lossy(&process.output.stderr),
        "report": add_runs(report, runs),
        "grade": grade,
        "witness": case.witness,
        "hole": case.hole,
        "detected_when": case.detected_when,
    })
}

fn expectation(expected: Expected) -> &'static str {
    match expected {
        Expected::Detect => "detect",
        Expected::NoAlarm => "no_alarm",
        Expected::ExpectedMiss => "expected_miss",
        Expected::Reference => "reference",
    }
}

fn legality_case(
    root: &Path,
    case: &Case,
    fixture: &Path,
    checker: &Path,
    tool: &Value,
) -> Result<Value> {
    let target = root.join("target/rha/module-fixture-checks");
    let legality_argv = vec![
        "cargo".to_owned(),
        "check".to_owned(),
        "--offline".to_owned(),
        "--locked".to_owned(),
        "--manifest-path".to_owned(),
        fixture.join("Cargo.toml").display().to_string(),
    ];
    let legality = run_process(root, &legality_argv, Some(&target))?;
    let legality_record = legality.json();
    if !legality.output.status.success() {
        let grade = finding_failure(
            grade::Grade::default(),
            "fixture legality cargo check failed; invalid Rust was not graded",
        );
        return Ok(case_record(
            case,
            &fixture
                .strip_prefix(root)
                .unwrap_or(fixture)
                .display()
                .to_string(),
            &legality,
            json!({"runs": [legality_record]}),
            grade,
            &[legality_record],
        ));
    }

    let architecture_argv = vec![
        checker.display().to_string(),
        "architecture".to_owned(),
        "--manifest-path".to_owned(),
        fixture.join("Cargo.toml").display().to_string(),
        "--module-rules".to_owned(),
        fixture.join("rha-modules.toml").display().to_string(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    let architecture = run_process(root, &architecture_argv, None)?;
    let report =
        serde_json::from_slice::<Value>(&architecture.output.stdout).unwrap_or(Value::Null);
    let validation = validate_report(root, fixture, &report, tool);
    let grade = module_grade(case, architecture.exit_status(), &report, validation);
    let runs = vec![legality_record, architecture.json()];
    Ok(case_record(
        case,
        &fixture
            .strip_prefix(root)
            .unwrap_or(fixture)
            .display()
            .to_string(),
        &architecture,
        report,
        grade,
        &runs,
    ))
}

fn site_case(root: &Path, case: &Case, checker: &Path, tool: &Value) -> Result<Value> {
    let fixture = root.join("crates/site");
    let argv = vec![
        checker.display().to_string(),
        "architecture".to_owned(),
        "--manifest-path".to_owned(),
        fixture.join("Cargo.toml").display().to_string(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    let process = run_process(root, &argv, None)?;
    let report = serde_json::from_slice::<Value>(&process.output.stdout).unwrap_or(Value::Null);
    let validation = validate_full_site_report(root, &report, tool);
    let grade = module_grade(case, process.exit_status(), &report, validation);
    let relative = fixture
        .strip_prefix(root)
        .unwrap_or(&fixture)
        .display()
        .to_string();
    let run = process.json();
    Ok(case_record(
        case,
        &relative,
        &process,
        report,
        grade,
        &[run],
    ))
}

fn random_package_hashes(root: &Path) -> Result<BTreeMap<String, String>> {
    let payloads = verify_inventory(root)?;
    let mut output = BTreeMap::new();
    let mut source_count = 0;
    for (relative, digest) in payloads {
        let is_random_payload = relative.starts_with("random/");
        let is_required_support_payload =
            relative == "random-rule-expectations.json" || relative == "reference.py";
        if is_random_payload || is_required_support_payload {
            if is_random_source_path(&relative) {
                source_count += 1;
            }
            output.insert(relative, digest);
        }
    }
    if source_count != 256 {
        return Err(Error::new(format!(
            "module random inventory contains {source_count} source maps, expected 256"
        )));
    }
    for required in ["random-rule-expectations.json", "reference.py"] {
        if !output.contains_key(required) {
            return Err(Error::new(format!(
                "module random inventory lacks required payload {required}"
            )));
        }
    }
    Ok(output)
}

fn is_random_source_path(relative: &str) -> bool {
    let Some(name) = relative.strip_prefix("random/") else {
        return false;
    };
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[0] == b'R'
        && bytes[1..5].iter().all(u8::is_ascii_digit)
        && &bytes[5..] == b".json"
}

fn registered_random_counts(root: &Path) -> Result<(Vec<u8>, BTreeMap<String, u64>)> {
    let path = root.join("registration.toml");
    let bytes = std::fs::read(&path).context(|| format!("reading {}", path.display()))?;
    let registration: toml::Value = toml::from_str(
        std::str::from_utf8(&bytes).context(|| format!("decoding {}", path.display()))?,
    )
    .context(|| format!("parsing {}", path.display()))?;
    let table = registration["counts"]
        .as_table()
        .ok_or_else(|| Error::new("module registration lacks counts table"))?;
    let mut counts = BTreeMap::new();
    for (key, value) in table {
        let count = value
            .as_integer()
            .filter(|count| *count >= 0)
            .ok_or_else(|| Error::new(format!("registration count {key} is not nonnegative")))?;
        counts.insert(key.clone(), count as u64);
    }
    Ok((bytes, counts))
}

fn validate_random_report_at(root: &Path, report: &Value) -> Result<()> {
    let registration_root = root.join(MODULE_ROOT);
    let (registration_bytes, registered_counts) = registered_random_counts(&registration_root)?;
    let registration_digest = sha256_hex(&registration_bytes);
    let registration_path = format!("{MODULE_ROOT}/registration.toml");
    let registration = report["registration"]
        .as_object()
        .ok_or_else(|| Error::new("random report lacks registration object"))?;
    if registration.get("path").and_then(Value::as_str) != Some(registration_path.as_str()) {
        return Err(Error::new(
            "random report references the wrong registration path",
        ));
    }
    if registration.get("sha256").and_then(Value::as_str) != Some(registration_digest.as_str())
        || report["digests"]["registration_sha256"].as_str() != Some(registration_digest.as_str())
    {
        return Err(Error::new(
            "random report registration digest does not match the registered file",
        ));
    }
    if registration.get("counts") != Some(&json!(registered_counts)) {
        return Err(Error::new(
            "random report registration counts differ from the registered counts",
        ));
    }

    let expected_ids = (0..256)
        .map(|index| format!("R{index:04}"))
        .collect::<Vec<_>>();
    if registration.get("ids") != Some(&json!(expected_ids)) {
        return Err(Error::new(
            "random report registration ids differ from R0000..R0255",
        ));
    }
    if report["outcome"] != "passed" {
        return Err(Error::new("random report outcome was not passed"));
    }

    let required_counts = [
        "random_cases",
        "random_edges",
        "random_test_edges",
        "random_heuristic_edges",
        "random_limitations",
        "optional_cycles",
        "optional_top_level_findings",
        "optional_top_level_undeclared_edges",
        "optional_subsumed_undeclared_edges",
        "optional_total_undeclared_edge_facts",
        "random_rust_files",
        "random_source_map_files",
    ];
    let counts = report["counts"]
        .as_object()
        .ok_or_else(|| Error::new("random report lacks counts object"))?;
    if counts.len() != required_counts.len() {
        return Err(Error::new(format!(
            "random report has {} count fields, expected {}",
            counts.len(),
            required_counts.len()
        )));
    }
    for key in required_counts {
        let expected = registered_counts
            .get(key)
            .ok_or_else(|| Error::new(format!("registration lacks required count {key}")))?;
        let actual = counts.get(key).and_then(Value::as_u64).ok_or_else(|| {
            Error::new(format!(
                "random report count {key} is absent or not an integer"
            ))
        })?;
        if actual != *expected {
            return Err(Error::new(format!(
                "random report count {key} is {actual}, expected registered value {expected}"
            )));
        }
    }

    let cases = report["cases"]
        .as_array()
        .ok_or_else(|| Error::new("random report lacks cases array"))?;
    if cases.len() != expected_ids.len() {
        return Err(Error::new(format!(
            "random report has {} cases, expected {}",
            cases.len(),
            expected_ids.len()
        )));
    }
    let mut actual_ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let id = case["id"]
            .as_str()
            .ok_or_else(|| Error::new(format!("random case {index} lacks string id")))?;
        if !expected_ids.iter().any(|expected| expected == id) {
            return Err(Error::new(format!(
                "random report contains unexpected case id {id}"
            )));
        }
        if !actual_ids.insert(id.to_owned()) {
            return Err(Error::new(format!("random report repeats case id {id}")));
        }
        if case["passed"] != true {
            return Err(Error::new(format!("random case {id} did not pass")));
        }
        let reasons = case["reasons"]
            .as_array()
            .ok_or_else(|| Error::new(format!("random case {id} lacks reasons array")))?;
        if !reasons.is_empty() {
            return Err(Error::new(format!("random case {id} has nonempty reasons")));
        }
    }
    if actual_ids != expected_ids.iter().cloned().collect::<BTreeSet<_>>() {
        return Err(Error::new(
            "random report case ids are not exactly R0000..R0255",
        ));
    }
    for field in ["mismatch_ids", "reasons"] {
        let values = report[field]
            .as_array()
            .ok_or_else(|| Error::new(format!("random report lacks {field} array")))?;
        if !values.is_empty() {
            return Err(Error::new(format!(
                "random report has nonempty aggregate {field}"
            )));
        }
    }
    Ok(())
}

/// Validate the complete supplementary random report.
pub fn validate_random_report(report: &Value) -> Result<()> {
    validate_random_report_at(&module_repository_root(), report)
}

fn random_case(root: &Path, fixture_hashes: &BTreeMap<String, String>) -> Result<Value> {
    let checker = crate::util::running_executable_path()?;
    let argv = vec![
        checker.display().to_string(),
        "corpus".to_owned(),
        "module-random".to_owned(),
    ];
    let process = run_process(root, &argv, None)?;
    let parsed = serde_json::from_slice::<Value>(&process.output.stdout).unwrap_or(Value::Null);
    let mut grade = grade::Grade::default();
    if process.exit_status() != Some(0) {
        grade
            .reasons
            .push("supplementary random checker exited nonzero".to_owned());
    }
    if let Err(error) = validate_random_report(&parsed) {
        grade
            .reasons
            .push(format!("invalid supplementary random report: {error}"));
    }
    grade.passed = grade.reasons.is_empty();
    let report = if parsed.is_object() {
        parsed
    } else {
        json!({"observed": parsed})
    };
    Ok(json!({
        "id": "random-module-reference",
        "cells": MODULE_CELLS,
        "expected": "supplementary",
        "detector": "xtask corpus module-random",
        "fixture_inputs_sha256": fixture_hashes,
        "argv": process.argv,
        "exit_status": process.exit_status(),
        "stdout": String::from_utf8_lossy(&process.output.stdout),
        "stderr": String::from_utf8_lossy(&process.output.stderr),
        "report": report,
        "grade": grade,
        "witness": {},
        "hole": null,
        "detected_when": null,
    }))
}

fn reference_complete(process: &Process) -> bool {
    let stdout = String::from_utf8_lossy(&process.output.stdout);
    let stderr = String::from_utf8_lossy(&process.output.stderr);
    match process.exit_status() {
        Some(0) => stdout.trim().starts_with("digraph {") && stdout.trim().ends_with('}'),
        Some(1) => stderr
            .lines()
            .any(|line| line.starts_with("Error: circular dependency between ")),
        _ => false,
    }
}

fn reference_case(root: &Path, m01: &Path, site: &Path) -> Result<Value> {
    let m01_argv = vec![
        "cargo".to_owned(),
        "modules".to_owned(),
        "dependencies".to_owned(),
        "--manifest-path".to_owned(),
        m01.join("Cargo.toml").display().to_string(),
        "--lib".to_owned(),
        "--acyclic".to_owned(),
    ];
    let site_argv = vec![
        "cargo".to_owned(),
        "modules".to_owned(),
        "dependencies".to_owned(),
        "--manifest-path".to_owned(),
        site.join("Cargo.toml").display().to_string(),
        "--package".to_owned(),
        "site".to_owned(),
        "--lib".to_owned(),
        "--acyclic".to_owned(),
    ];
    let version_argv = vec![
        "cargo".to_owned(),
        "modules".to_owned(),
        "--version".to_owned(),
    ];
    let target = root.join("target/rha/module-reference");
    let m01_run = run_process(root, &m01_argv, Some(&target))?;
    let site_run = run_process(root, &site_argv, Some(&target))?;
    let version_run = run_process(root, &version_argv, None)?;
    let runs = vec![m01_run.json(), site_run.json(), version_run.json()];
    let mut grade = grade::Grade::default();
    if !reference_complete(&m01_run) || !reference_complete(&site_run) {
        grade
            .reasons
            .push("cargo-modules reference did not complete both analyses".to_owned());
    }
    if version_run.exit_status() != Some(0) {
        grade
            .reasons
            .push("cargo-modules version command did not exit successfully".to_owned());
    }
    if String::from_utf8_lossy(&version_run.output.stdout).trim() != "cargo-modules 0.27.0" {
        grade.reasons.push(
            "cargo-modules version output differed from the measured registration".to_owned(),
        );
    }
    grade.passed = grade.reasons.is_empty();
    let first = numeric_exit(&m01_run, "cargo modules M01 reference")?;
    let mut input_hashes = BTreeMap::new();
    let m01_inputs = fixture_digests(m01)?;
    if m01_inputs.is_empty() {
        return Err(Error::new(
            "M01 reference fixture input digest map is empty",
        ));
    }
    for (relative, digest) in m01_inputs {
        input_hashes.insert(format!("M01/{relative}"), digest);
    }
    let site_inputs = fixture_digests(site)?
        .into_iter()
        .filter(|(relative, _)| {
            relative == "Cargo.toml"
                || relative == "rha-modules.toml"
                || relative.starts_with("src/")
        })
        .collect::<BTreeMap<_, _>>();
    if site_inputs.is_empty() {
        return Err(Error::new(
            "site reference fixture input digest map is empty",
        ));
    }
    for (relative, digest) in site_inputs {
        input_hashes.insert(format!("site/{relative}"), digest);
    }
    Ok(json!({
        "id": "X-M01",
        "cells": ["law6-b3"],
        "expected": "reference",
        "detector": "cargo modules",
        "fixture_inputs_sha256": input_hashes,
        "argv": m01_run.argv,
        "exit_status": first,
        "stdout": String::from_utf8_lossy(&m01_run.output.stdout),
        "stderr": String::from_utf8_lossy(&m01_run.output.stderr),
        "report": {
            "runs": runs,
            "limits": [
                "cargo-modules reports an item graph; it is not the component-collapse graph used by the module checker",
                "reference completion records command completion, not agreement with the registered module graph"
            ]
        },
        "grade": grade,
        "witness": {},
        "hole": null,
        "detected_when": null,
    }))
}

fn pre_registration_revision(root: &Path) -> Result<String> {
    let text = std::fs::read_to_string(root.join(".rha/acceptances/CHG-002.toml"))
        .context(|| "reading CHG-002 acceptance identity".to_owned())?;
    let value: toml::Value =
        toml::from_str(&text).context(|| "parsing CHG-002 acceptance identity".to_owned())?;
    let revision = value["subject"]["revision"]
        .as_str()
        .ok_or_else(|| Error::new("CHG-002 acceptance lacks subject.revision"))?;
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::new(
            "CHG-002 subject.revision is not a full revision",
        ));
    }
    Ok(revision.to_owned())
}

fn evidence_directory(root: &Path, args: &CorpusArgs) -> PathBuf {
    if args.evidence == Path::new("evidence/h4-crate") {
        root.join("evidence/h4-module")
    } else {
        root.join(&args.evidence)
    }
}

/// Run and grade the frozen module corpus.
pub fn run(root: &Path, args: &CorpusArgs) -> Result<u8> {
    let module_root = root.join(MODULE_ROOT);
    let payloads = verify_inventory(&module_root)?;
    verify_registration_contract(&module_root, &payloads)?;
    let (manifest, cases) = manifest_module_cases(root)?;
    let fixtures = headline_fixtures(&module_root)?;

    let now = UtcTime::now();
    let subject =
        crate::evidence::subject::capture(root, &root.join("target/rha/h4-module-subject"))?;
    let checker = crate::util::running_executable_path()?;
    let tool = crate::graph::report::tool_identity(root);
    let authored = cases
        .iter()
        .filter(|case| AUTHORED_IDS.contains(&case.id.as_str()))
        .collect::<Vec<_>>();
    if authored.len() != 25 {
        return Err(Error::new(
            "module corpus did not resolve 25 authored cases",
        ));
    }

    let mut headline_records = Vec::new();
    for case in authored {
        let fixture = fixtures
            .get(&case.id)
            .ok_or_else(|| Error::new(format!("missing fixture {}", case.id)))?;
        let observed = legality_case(root, case, fixture, &checker, &tool)?;
        println!(
            "{}: {}",
            case.id,
            if observed["grade"]["passed"] == true {
                "passed"
            } else {
                "failed"
            }
        );
        headline_records.push(observed);
    }

    let live_case = cases
        .iter()
        .find(|case| case.id == "L-M01")
        .ok_or_else(|| Error::new("missing L-M01 registration"))?;
    headline_records.push(site_case(root, live_case, &checker, &tool)?);

    let m01 = fixtures
        .get("M01")
        .ok_or_else(|| Error::new("missing M01 fixture"))?;
    let site = root.join("crates/site");
    let reference_record = reference_case(root, m01, &site)?;

    let random_hashes = random_package_hashes(&module_root)?;
    let supplementary_record = random_case(root, &random_hashes)?;

    let mut records = headline_records.clone();
    records.push(reference_record);
    records.push(supplementary_record);

    let mut summary = runner::summarize(&headline_records);
    let failed_cases = records
        .iter()
        .filter(|record| record["grade"]["passed"] != true)
        .filter_map(|record| record["id"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    summary["outcome"] = json!(if failed_cases.is_empty() {
        "passed"
    } else {
        "failed"
    });
    summary["failed_cases"] = json!(failed_cases);

    let mut downgraded = BTreeSet::new();
    for record in &records {
        if record["grade"]["passed"] != true {
            if record["detector"] == "xtask corpus module-random" {
                downgraded.extend(MODULE_CELLS);
            } else {
                downgraded.extend(
                    record["cells"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                );
            }
        }
    }

    let fixture_inputs = fixture_digests(&module_root.join("headline"))?;
    let manifest_sha = sha256_file(&root.join(MANIFEST_PATH))?;
    let template_sha = sha256_file(&root.join(crate::clippy_template::TEMPLATE_PATH))?;
    let amendments = runner::amendments(root)?;
    let record = json!({
        "schema_version": 2,
        "kind": "h4",
        "level": "module",
        "evidence_class": "local",
        "advisory": true,
        "created_at": now.rfc3339(),
        "artifact_identity": subject,
        "producer": {
            "name": "xtask corpus run",
            "version": env!("CARGO_PKG_VERSION"),
            "executable_sha256": crate::util::running_executable_sha256()?,
            "checker": tool,
        },
        "pre_registration": {
            "change": "CHG-002",
            "identity": manifest.pre_registration,
            "revision": pre_registration_revision(root)?,
            "record": ".rha/acceptances/CHG-002.toml",
        },
        "manifest": {
            "path": MANIFEST_PATH,
            "sha256": manifest_sha,
            "amendments": amendments,
            "correction_commit": amendments.last().map_or(Value::Null, |item| item["commit"].clone()),
        },
        "fixtures": {
            "root": MODULE_ROOT,
            "drift": [],
            "inputs_sha256": fixture_inputs,
        },
        "template_sha256": template_sha,
        "grading": {
            "decision": "DP-1.1c",
            "detection_requires": manifest.grading.detection_requires,
            "extra_findings": manifest.grading.extra_findings,
            "no_alarm_scope": manifest.grading.no_alarm_scope,
            "expected_miss_surprise": manifest.grading.expected_miss_surprise,
        },
        "environment": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cargo": command_stdout(root, &["cargo", "--version"])?,
            "rustc": command_stdout(root, &["rustc", "--version"])?,
        },
        "summary": summary,
        "cases": records,
        "downgraded_cells": downgraded,
        "held_out": {
            "outcome": "not_run",
            "reason": "Kennedy-owned; no agent reads or runs held-out material",
        },
    });
    let issues = crate::record_schema::validate(&record, "h4");
    if !issues.is_empty() {
        return Err(Error::new(format!(
            "module H4 record failed schema validation: {issues:?}"
        )));
    }

    let directory = evidence_directory(root, args);
    std::fs::create_dir_all(&directory)
        .context(|| format!("creating module evidence directory {}", directory.display()))?;
    let path = directory.join(format!(
        "{}-{}.json",
        now.compact(),
        &subject.revision[..12]
    ));
    let bytes = serde_json::to_vec_pretty(&record)
        .context(|| "serializing module H4 evidence".to_owned())?;
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .context(|| format!("creating immutable module evidence {}", path.display()))?;
    file.write_all(&bytes)
        .context(|| "writing immutable module evidence".to_owned())?;
    let passed_count = records
        .iter()
        .filter(|record| record["grade"]["passed"] == true)
        .count();
    let failed_count = records.len() - passed_count;
    let case_outcome = |id: &str| {
        records
            .iter()
            .find(|record| record["id"] == id)
            .map_or(Value::Null, |record| record["grade"]["passed"].clone())
    };
    println!(
        "module H4: outcome={}, evidence={}, cases={}, passed={}, failed={}, live={}, reference={}, supplementary={}",
        summary["outcome"].as_str().unwrap_or("unknown"),
        path.strip_prefix(root).unwrap_or(&path).display(),
        records.len(),
        passed_count,
        failed_count,
        case_outcome("L-M01"),
        case_outcome("X-M01"),
        case_outcome("random-module-reference"),
    );
    Ok(u8::from(!failed_cases.is_empty()))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    fn process(code: i32, stdout: &str, stderr: &str) -> Process {
        Process {
            argv: vec!["cargo".to_owned(), "modules".to_owned()],
            output: Output {
                status: ExitStatus::from_raw(code << 8),
                stdout: stdout.as_bytes().to_vec(),
                stderr: stderr.as_bytes().to_vec(),
            },
        }
    }

    #[test]
    fn reference_completion_requires_the_known_cargo_modules_shapes() {
        assert!(reference_complete(&process(
            0,
            "digraph {\n  \"a\" -> \"b\";\n}\n",
            ""
        )));
        assert!(reference_complete(&process(
            1,
            "",
            "Error: circular dependency between constraints and ordering\n"
        )));
        assert!(!reference_complete(&process(
            1,
            "",
            "Error: module dependency cycle was not resolved\n"
        )));
        assert!(!reference_complete(&process(
            2,
            "",
            "Usage: cargo modules dependencies\n"
        )));
        assert!(!reference_complete(&process(0, "", "")));
    }
}
