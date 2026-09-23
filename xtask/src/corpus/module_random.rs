//! Supplementary differential corpus for the independently generated module cases.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::error::{Context as _, Error, Result};
use crate::modules::{extract, rules};
use crate::util::sha256_hex;

const MODULE_ROOT: &str = "xtask/tests/corpus/module";
const RANDOM_DIR: &str = "xtask/tests/corpus/module/random";
const REGISTRATION: &str = "xtask/tests/corpus/module/registration.toml";
const RULE_EXPECTATIONS: &str = "xtask/tests/corpus/module/random-rule-expectations.json";

#[derive(Debug, Deserialize)]
struct SourceMap {
    id: String,
    crate_name: String,
    edition: String,
    files: BTreeMap<String, String>,
}

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(root: &Path) -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let root = fs::canonicalize(root)
            .context(|| format!("canonicalizing workspace root {}", root.display()))?;
        let parent = root.join("target/rha");
        fs::create_dir_all(&parent)
            .context(|| format!("creating module-random scratch parent {}", parent.display()))?;
        let parent = fs::canonicalize(&parent)
            .context(|| format!("canonicalizing scratch parent {}", parent.display()))?;
        if !parent.starts_with(&root) {
            return Err(Error::new(format!(
                "module-random scratch parent escapes workspace root: {}",
                parent.display()
            )));
        }

        for _ in 0..256 {
            let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("module-random-{}-{nonce}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(Error::new(format!(
                        "creating module-random scratch directory {}: {error}",
                        path.display()
                    )));
                }
            }
        }

        Err(Error::new(
            "could not allocate a unique module-random scratch directory",
        ))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path)
            && self.path.exists()
        {
            eprintln!(
                "failed to remove module-random scratch directory {}: {error}",
                self.path.display()
            );
        }
    }
}

#[derive(Default)]
struct CorpusFiles {
    sources: BTreeMap<String, PathBuf>,
    expected: BTreeMap<String, PathBuf>,
    unknown: Vec<String>,
}

fn is_case_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 5 && bytes[0] == b'R' && bytes[1..].iter().all(u8::is_ascii_digit)
}

fn classify_name(name: &str) -> Option<(&'static str, String)> {
    if let Some(stem) = name.strip_suffix(".expected.json")
        && is_case_id(stem)
    {
        return Some(("expected", stem.to_owned()));
    }
    if let Some(stem) = name.strip_suffix(".json")
        && is_case_id(stem)
    {
        return Some(("source", stem.to_owned()));
    }
    None
}

fn scan_random_dir(path: &Path) -> Result<CorpusFiles> {
    let mut files = CorpusFiles::default();
    for entry in fs::read_dir(path).context(|| format!("listing {}", path.display()))? {
        let entry = entry.context(|| format!("reading entry in {}", path.display()))?;
        let entry_path = entry.path();
        let display = entry_path.display().to_string();
        let file_type = entry
            .file_type()
            .context(|| format!("reading file type for {display}"))?;
        if file_type.is_symlink() {
            files.unknown.push(format!("{display}: symlink"));
            continue;
        }
        if !file_type.is_file() {
            files
                .unknown
                .push(format!("{display}: unexpected non-file"));
            continue;
        }

        let Some(name) = entry_path.file_name().and_then(|name| name.to_str()) else {
            files.unknown.push(format!("{display}: non-UTF-8 filename"));
            continue;
        };
        match classify_name(name) {
            Some(("source", id)) => {
                files.sources.insert(id, entry_path);
            }
            Some(("expected", id)) => {
                files.expected.insert(id, entry_path);
            }
            _ => files.unknown.push(format!("{display}: unexpected file")),
        }
    }
    Ok(files)
}

fn safe_relative(path: &str) -> std::result::Result<&Path, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return Err("empty relative path".to_owned());
    }
    if path.is_absolute() {
        return Err(format!("absolute path is not allowed: {}", path.display()));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "parent, prefix, root, or other non-normal component is not allowed: {}",
            path.display()
        ));
    }
    Ok(path)
}

fn materialize(root: &Path, files: &BTreeMap<String, String>) -> std::result::Result<(), String> {
    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("{}: {error}", root.display()))?;

    for (relative, contents) in files {
        let relative = safe_relative(relative).map_err(|error| format!("{relative}: {error}"))?;
        let target = root.join(relative);
        let parent = target
            .parent()
            .ok_or_else(|| format!("{} has no parent", target.display()))?;
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        let canonical_parent =
            fs::canonicalize(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(format!("symlink escape through {}", parent.display()));
        }
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && metadata.file_type().is_symlink()
        {
            return Err(format!("refusing symlink target {}", target.display()));
        }
        if !target.starts_with(root) {
            return Err(format!(
                "materialized path escaped root: {}",
                target.display()
            ));
        }
        fs::write(&target, contents).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(())
}

fn registration_counts(value: &toml::Value) -> Result<BTreeMap<String, usize>> {
    let table = value
        .get("counts")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| Error::new("module registration lacks [counts]"))?;
    let mut counts = BTreeMap::new();
    for (key, value) in table {
        let count = value
            .as_integer()
            .ok_or_else(|| Error::new(format!("registration counts.{key} is not an integer")))?;
        let count = usize::try_from(count)
            .map_err(|_| Error::new(format!("registration counts.{key} is negative")))?;
        counts.insert(key.clone(), count);
    }
    Ok(counts)
}

fn required_count(counts: &BTreeMap<String, usize>, key: &str) -> Result<usize> {
    counts
        .get(key)
        .copied()
        .ok_or_else(|| Error::new(format!("registration is missing counts.{key}")))
}

fn add_count(counts: &mut BTreeMap<String, usize>, key: &str, amount: usize) {
    *counts.entry(key.to_owned()).or_default() += amount;
}

fn registered_ids() -> BTreeSet<String> {
    (0..256).map(|index| format!("R{index:04}")).collect()
}

fn read_json_map(path: &Path) -> Result<BTreeMap<String, Value>> {
    let bytes = fs::read(path).context(|| format!("reading {}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| Error::new(format!("parsing {}: {error}", path.display())))?;
    let object = value
        .as_object()
        .ok_or_else(|| Error::new(format!("{} must contain a JSON object", path.display())))?;
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

fn component_paths(path: &Path) -> std::result::Result<BTreeMap<String, Vec<String>>, String> {
    let bytes = fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;
    let value: toml::Value =
        toml::from_slice(&bytes).map_err(|error| format!("parsing {}: {error}", path.display()))?;
    let components = value
        .get("components")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "module rules lack [components]".to_owned())?;
    let mut result = BTreeMap::new();
    for (alias, path) in components {
        let path = path
            .as_str()
            .ok_or_else(|| format!("component {alias} path is not a string"))?;
        let parts = path.split("::").map(ToOwned::to_owned).collect::<Vec<_>>();
        if parts.is_empty() || parts.iter().any(String::is_empty) {
            return Err(format!("component {alias} has an invalid path"));
        }
        result.insert(alias.clone(), parts);
    }
    Ok(result)
}

fn is_prefix(prefix: &[String], value: &[String]) -> bool {
    value.starts_with(prefix)
}

fn owner<'a>(path: &[String], components: &'a BTreeMap<String, Vec<String>>) -> Option<&'a str> {
    components
        .iter()
        .filter(|(_, component)| is_prefix(component, path))
        .max_by(|left, right| {
            left.1
                .len()
                .cmp(&right.1.len())
                .then_with(|| right.0.cmp(left.0))
        })
        .map(|(alias, _)| alias.as_str())
}

fn path_parts(path: &str) -> Vec<String> {
    path.split("::").map(ToOwned::to_owned).collect()
}

fn collapsed_edges(
    graph: &extract::Extracted,
    components: &BTreeMap<String, Vec<String>>,
) -> Value {
    let mut edges = BTreeSet::new();
    for edge in graph.edges.iter().filter(|edge| !edge.test_only) {
        let source = owner(&path_parts(&edge.source), components);
        let target = owner(&path_parts(&edge.target), components);
        if let (Some(source), Some(target)) = (source, target)
            && source != target
        {
            edges.insert((
                source.to_owned(),
                target.to_owned(),
                edge.extraction.clone(),
            ));
        }
    }
    Value::Array(
        edges
            .into_iter()
            .map(|(source, target, extraction)| {
                json!({
                    "source": source,
                    "target": target,
                    "extraction": extraction,
                })
            })
            .collect(),
    )
}

fn canonical_diagnostic(value: &Value, crate_name: &str) -> Value {
    match value {
        Value::String(value) => value
            .strip_prefix("crate::")
            .map(|rest| Value::String(format!("{crate_name}::{rest}")))
            .unwrap_or_else(|| value.clone().into()),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| canonical_diagnostic(value, crate_name))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), canonical_diagnostic(value, crate_name)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn matches_expected(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            expected.iter().all(|(key, expected)| {
                actual
                    .get(key)
                    .is_some_and(|actual| matches_expected(actual, expected))
            })
        }
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| matches_expected(actual, expected))
        }
        _ => actual == expected,
    }
}

fn findings_match(actual: &[Value], expected: &[Value], crate_name: &str) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let actual = actual
        .iter()
        .map(|value| canonical_diagnostic(value, crate_name))
        .collect::<Vec<_>>();
    let mut used = vec![false; actual.len()];
    expected.iter().all(|expected| {
        actual
            .iter()
            .enumerate()
            .find(|(index, actual)| !used[*index] && matches_expected(actual, expected))
            .map(|(index, _)| {
                used[index] = true;
                true
            })
            .unwrap_or(false)
    })
}

fn array_field<'a>(value: &'a Value, key: &str) -> Option<&'a [Value]> {
    value.get(key).and_then(Value::as_array).map(Vec::as_slice)
}

fn case_observation(
    id: &str,
    source_path: Option<&Path>,
    expected_path: Option<&Path>,
    optional: Option<&BTreeMap<String, Value>>,
    scratch: &Path,
    counts: &mut BTreeMap<String, usize>,
) -> Value {
    let mut reasons = Vec::new();
    let mut actual = Map::new();

    let Some(source_path) = source_path else {
        reasons.push("source map is missing".to_owned());
        return json!({"id": id, "passed": false, "reasons": reasons, "actual": {"error": "source map is missing"}});
    };
    let Some(expected_path) = expected_path else {
        reasons.push("expected output is missing".to_owned());
        return json!({"id": id, "passed": false, "reasons": reasons, "actual": {"error": "expected output is missing"}});
    };

    let source_bytes = match fs::read(source_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let message = format!("reading source map: {error}");
            return json!({"id": id, "passed": false, "reasons": [message.clone()], "actual": {"error": message}});
        }
    };
    let source: SourceMap = match serde_json::from_slice(&source_bytes) {
        Ok(source) => source,
        Err(error) => {
            let message = format!("invalid source map: {error}");
            return json!({"id": id, "passed": false, "reasons": [message.clone()], "actual": {"error": message}});
        }
    };
    add_count(counts, "random_source_map_files", source.files.len());
    add_count(
        counts,
        "random_rust_files",
        source
            .files
            .keys()
            .filter(|path| path.ends_with(".rs"))
            .count(),
    );
    actual.insert(
        "source".to_owned(),
        json!({
            "id": source.id,
            "crate_name": source.crate_name,
            "edition": source.edition,
            "files": source.files.len(),
        }),
    );
    if source.id != id {
        reasons.push(format!("source map id is {}, expected {id}", source.id));
    }

    let expected = match fs::read(expected_path) {
        Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => Some(value),
            Err(error) => {
                reasons.push(format!("invalid expected output: {error}"));
                None
            }
        },
        Err(error) => {
            reasons.push(format!("reading expected output: {error}"));
            None
        }
    };

    let case_root = scratch.join(id);
    match materialize(&case_root, &source.files) {
        Ok(()) => {}
        Err(error) => {
            reasons.push(format!("materialization failed: {error}"));
            actual.insert("error".to_owned(), json!(error));
            return json!({
                "id": id,
                "passed": reasons.is_empty(),
                "reasons": reasons,
                "actual": actual,
            });
        }
    }

    let extracted = match extract::extract(
        &case_root,
        &case_root.join("src/lib.rs"),
        &source.crate_name,
        &source.edition,
        &BTreeSet::new(),
    ) {
        Ok(extracted) => extracted,
        Err(error) => {
            reasons.push(format!("extractor returned error: {error}"));
            actual.insert("error".to_owned(), json!(error));
            return json!({
                "id": id,
                "passed": false,
                "reasons": reasons,
                "actual": actual,
            });
        }
    };

    add_count(counts, "random_edges", extracted.edges.len());
    add_count(
        counts,
        "random_test_edges",
        extracted.edges.iter().filter(|edge| edge.test_only).count(),
    );
    add_count(
        counts,
        "random_heuristic_edges",
        extracted
            .edges
            .iter()
            .filter(|edge| edge.extraction == "heuristic")
            .count(),
    );
    add_count(counts, "random_limitations", extracted.limitations.len());

    let extraction = json!({
        "schema_version": 1,
        "edges": extracted.edges,
        "limitations": extracted.limitations,
    });
    actual.insert("extraction".to_owned(), extraction.clone());
    if let Some(expected) = expected
        && extraction != expected
    {
        reasons.push("extraction differs from committed expected output".to_owned());
    }

    let Some(optional) = optional else {
        return json!({
            "id": id,
            "passed": reasons.is_empty(),
            "reasons": reasons,
            "actual": actual,
        });
    };
    let Some(rule_expected) = optional.get(id) else {
        reasons.push("optional rule expectation is missing".to_owned());
        return json!({
            "id": id,
            "passed": false,
            "reasons": reasons,
            "actual": actual,
        });
    };

    let rules_path = case_root.join("rha-modules.toml");
    let components = match component_paths(&rules_path) {
        Ok(components) => components,
        Err(error) => {
            reasons.push(format!("reading component projection: {error}"));
            actual.insert("rules".to_owned(), json!({"error": error}));
            return json!({
                "id": id,
                "passed": false,
                "reasons": reasons,
                "actual": actual,
            });
        }
    };
    let checked = match rules::check(&case_root, &source.crate_name, &rules_path, &extracted) {
        Ok(checked) => checked,
        Err(error) => {
            reasons.push(format!("module rules returned error: {error}"));
            actual.insert("rules".to_owned(), json!({"error": error}));
            return json!({
                "id": id,
                "passed": false,
                "reasons": reasons,
                "actual": actual,
            });
        }
    };

    let finding_values = checked.findings.clone();
    let top_level_undeclared = finding_values
        .iter()
        .filter(|finding| finding["rule"] == "modules.undeclared_dependency")
        .count();
    let subsumed_undeclared = finding_values
        .iter()
        .filter_map(|finding| finding.get("undeclared_edges"))
        .filter_map(Value::as_array)
        .map(Vec::len)
        .sum::<usize>();
    add_count(counts, "optional_top_level_findings", finding_values.len());
    add_count(
        counts,
        "optional_cycles",
        finding_values
            .iter()
            .filter(|finding| finding["rule"] == "modules.cycle")
            .count(),
    );
    add_count(
        counts,
        "optional_top_level_undeclared_edges",
        top_level_undeclared,
    );
    add_count(
        counts,
        "optional_subsumed_undeclared_edges",
        subsumed_undeclared,
    );
    add_count(
        counts,
        "optional_total_undeclared_edge_facts",
        top_level_undeclared + subsumed_undeclared,
    );

    let component_edges = collapsed_edges(&extracted, &components);
    actual.insert(
        "rules".to_owned(),
        json!({
            "component_edges": component_edges,
            "violations": checked.findings,
            "test_edges": checked.test_edges,
            "module_edges": checked.module_edges,
            "limitations": checked.limitations,
        }),
    );

    let Some(expected_components) = rule_expected.get("component_edges") else {
        reasons.push("optional rule expectation lacks component_edges".to_owned());
        return json!({
            "id": id,
            "passed": false,
            "reasons": reasons,
            "actual": actual,
        });
    };
    if actual["rules"]["component_edges"] != *expected_components {
        reasons.push("component edge projection differs from committed expectation".to_owned());
    }

    let Some(expected_findings) = array_field(rule_expected, "violations") else {
        reasons.push("optional rule expectation lacks violations array".to_owned());
        return json!({
            "id": id,
            "passed": false,
            "reasons": reasons,
            "actual": actual,
        });
    };
    if !findings_match(&finding_values, expected_findings, &source.crate_name) {
        reasons.push("production diagnostics differ from committed rule expectation".to_owned());
    }

    json!({
        "id": id,
        "passed": reasons.is_empty(),
        "reasons": reasons,
        "actual": actual,
    })
}

pub fn run(root: &Path) -> Result<Value> {
    let payloads = super::module::verify_inventory(&root.join(MODULE_ROOT))?;
    super::module::verify_registration_contract(&root.join(MODULE_ROOT), &payloads)?;
    let registration_path = root.join(REGISTRATION);
    let registration_bytes = fs::read(&registration_path)
        .context(|| format!("reading {}", registration_path.display()))?;
    let registration_text = String::from_utf8(registration_bytes.clone()).map_err(|error| {
        Error::new(format!(
            "{} is not UTF-8: {error}",
            registration_path.display()
        ))
    })?;
    let registration: toml::Value = toml::from_str(&registration_text)
        .map_err(|error| Error::new(format!("parsing {}: {error}", registration_path.display())))?;
    let registered_counts = registration_counts(&registration)?;

    let random_path = root.join(RANDOM_DIR);
    let files = scan_random_dir(&random_path)?;
    let source_ids = files.sources.keys().cloned().collect::<BTreeSet<_>>();
    let expected_ids = files.expected.keys().cloned().collect::<BTreeSet<_>>();
    let all_ids = source_ids
        .union(&expected_ids)
        .cloned()
        .collect::<BTreeSet<_>>();
    let pinned_ids = registered_ids();
    let mut global_reasons = files
        .unknown
        .iter()
        .map(|path| format!("unexpected random corpus entry: {path}"))
        .collect::<Vec<_>>();

    if source_ids != expected_ids {
        global_reasons.push("source map and expected-output id sets differ".to_owned());
    }
    if source_ids != pinned_ids {
        global_reasons
            .push("source map ids differ from the registered R0000..R0255 set".to_owned());
    }

    let optional_path = root.join(RULE_EXPECTATIONS);
    let optional = match fs::symlink_metadata(&optional_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(Error::new(format!(
                "refusing symlink optional rule expectations {}",
                optional_path.display()
            )));
        }
        Ok(_) => Some(read_json_map(&optional_path)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::new("registered rule oracle is missing"));
        }
        Err(error) => {
            return Err(Error::new(format!(
                "reading optional rule expectation metadata {}: {error}",
                optional_path.display()
            )));
        }
    };

    if let Some(optional) = &optional {
        let optional_ids = optional.keys().cloned().collect::<BTreeSet<_>>();
        if optional_ids != source_ids {
            global_reasons
                .push("optional rule expectation ids differ from the source map id set".to_owned());
        }
    }

    let scratch = Scratch::new(root)?;
    let mut actual_counts = BTreeMap::new();
    actual_counts.insert("random_cases".to_owned(), source_ids.len());
    let mut cases = Vec::new();
    for id in &all_ids {
        cases.push(case_observation(
            id,
            files.sources.get(id).map(PathBuf::as_path),
            files.expected.get(id).map(PathBuf::as_path),
            optional.as_ref(),
            &scratch.path,
            &mut actual_counts,
        ));
    }

    let compare_keys = [
        "random_cases",
        "random_source_map_files",
        "random_rust_files",
        "random_edges",
        "random_test_edges",
        "random_heuristic_edges",
        "random_limitations",
        "optional_top_level_findings",
        "optional_cycles",
        "optional_top_level_undeclared_edges",
        "optional_subsumed_undeclared_edges",
        "optional_total_undeclared_edge_facts",
    ];
    for key in compare_keys {
        if optional.is_none() && key.starts_with("optional_") {
            continue;
        }
        let actual = actual_counts.get(key).copied().unwrap_or_default();
        let registered = required_count(&registered_counts, key)?;
        if actual != registered {
            global_reasons.push(format!(
                "count mismatch for {key}: actual {actual}, registered {registered}"
            ));
        }
    }

    for (key, expected) in [
        ("random_cases", 256),
        ("random_edges", 5229),
        ("random_test_edges", 512),
        ("random_heuristic_edges", 116),
        ("random_limitations", 0),
        ("optional_top_level_findings", 192),
        ("optional_total_undeclared_edge_facts", 128),
    ] {
        if required_count(&registered_counts, key)? != expected {
            global_reasons.push(format!(
                "registration count {key} is not pinned to {expected}"
            ));
        }
    }

    let mismatch_ids = cases
        .iter()
        .filter(|case| case["passed"] != true)
        .filter_map(|case| case["id"].as_str())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let case_failed = !mismatch_ids.is_empty();
    let outcome = if global_reasons.is_empty() && !case_failed {
        "passed"
    } else {
        "failed"
    };

    let mut digests = BTreeMap::new();
    digests.insert(
        "registration_sha256".to_owned(),
        sha256_hex(&registration_bytes),
    );
    if let Some(optional) = &optional {
        let bytes =
            fs::read(&optional_path).context(|| format!("reading {}", optional_path.display()))?;
        let _ = optional;
        digests.insert("rule_expectations_sha256".to_owned(), sha256_hex(&bytes));
    }

    Ok(json!({
        "outcome": outcome,
        "registration": {
            "path": REGISTRATION,
            "sha256": digests["registration_sha256"],
            "counts": registered_counts,
            "ids": pinned_ids,
        },
        "paths": {
            "module_root": MODULE_ROOT,
            "random": RANDOM_DIR,
            "rule_expectations": RULE_EXPECTATIONS,
        },
        "digests": digests,
        "counts": actual_counts,
        "cases": cases,
        "mismatch_ids": mismatch_ids,
        "reasons": global_reasons,
        "limits": [
            "supplementary module corpus; it does not change headline corpus grading",
            "committed independent expectations are compared as observations and are never rebuilt",
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_arrays_are_ordered_but_unregistered_object_keys_are_ignored() {
        let expected = json!({
            "closed_path": ["a", "b", "a"],
            "members": ["a", "b"],
            "subfacts": [
                {"kind": "edge", "path": ["a", "b"]},
                {"kind": "edge", "path": ["b", "a"]}
            ]
        });

        assert!(matches_expected(&expected, &expected));

        let mut extra_object_key = expected.clone();
        extra_object_key
            .as_object_mut()
            .expect("expected object")
            .insert("unregistered".to_owned(), json!("allowed"));
        assert!(matches_expected(&extra_object_key, &expected));

        let mut same_multiset_bad_closed_path = expected.clone();
        same_multiset_bad_closed_path["closed_path"] = json!(["a", "a", "b"]);
        assert!(!matches_expected(&same_multiset_bad_closed_path, &expected));

        let mut members_order_bad = expected.clone();
        members_order_bad["members"] = json!(["b", "a"]);
        assert!(!matches_expected(&members_order_bad, &expected));

        let mut nested_subfacts_order_bad = expected.clone();
        nested_subfacts_order_bad["subfacts"]
            .as_array_mut()
            .expect("subfacts array")
            .swap(0, 1);
        assert!(!matches_expected(&nested_subfacts_order_bad, &expected));
    }

    #[test]
    fn findings_match_reorders_only_the_top_level_findings() {
        let first = json!({"rule": "cycle", "path": ["a", "b", "a"]});
        let second = json!({"rule": "edge", "path": ["a", "b"]});
        assert!(findings_match(
            &[second.clone(), first.clone()],
            &[first.clone(), second.clone()],
            "fixture"
        ));

        let nested_order_bad = json!({"rule": "cycle", "path": ["a", "a", "b"]});
        assert!(!findings_match(&[nested_order_bad], &[first], "fixture"));
    }
}
