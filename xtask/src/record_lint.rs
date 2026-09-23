//! `cargo xtask rha lint`: structural and semantic record linting.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::record_schema;
use crate::util::sha256_hex;

fn is_hex(s: &str, n: usize, lower: bool) -> bool {
    s.len() == n
        && s.bytes().all(|b| {
            b.is_ascii_digit()
                || (b'a'..=b'f').contains(&b)
                || (!lower && (b'A'..=b'F').contains(&b))
        })
}

fn digest_ok(v: &str, unknown_allowed: bool) -> bool {
    (unknown_allowed && v == "unknown")
        || is_hex(v, 64, true)
        || v.strip_prefix("sha256:")
            .is_some_and(|h| is_hex(h, 64, true))
}

/// RFC 3339 with a real calendar date, a valid time, and `Z` or `±HH:MM`.
fn rfc3339(v: &str) -> bool {
    let b = v.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return false;
    }
    let num = |r: std::ops::Range<usize>| -> Option<u32> {
        let s = b.get(r)?;
        s.iter()
            .all(u8::is_ascii_digit)
            .then(|| s.iter().fold(0u32, |n, d| n * 10 + u32::from(d - b'0')))
    };
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi), Some(se)) = (
        num(0..4),
        num(5..7),
        num(8..10),
        num(11..13),
        num(14..16),
        num(17..19),
    ) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = match mo {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    if d < 1 || d > days || h > 23 || mi > 59 || se > 60 {
        return false;
    }
    let mut i = 19;
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        i = start;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    match &b[i..] {
        [b'Z' | b'z'] => true,
        [b'+' | b'-', h1, h2, b':', m1, m2]
            if [h1, h2, m1, m2].iter().all(|c| c.is_ascii_digit()) =>
        {
            (h1 - b'0') * 10 + (h2 - b'0') <= 23 && (m1 - b'0') * 10 + (m2 - b'0') <= 59
        }
        _ => false,
    }
}

fn is_digest_key(k: &str) -> bool {
    k != "digest_algorithm" && (k.contains("sha256") || k.ends_with("digest"))
}

fn is_revision_key(k: &str) -> bool {
    matches!(k, "revision" | "tree" | "commit" | "git_rev")
        || k.ends_with("_revision")
        || k.ends_with("_tree")
        || k.ends_with("_git_rev")
}

struct Cited<'a> {
    dir: PathBuf,
    canonical_dir: Option<PathBuf>,
    repo: Option<&'a Path>,
    canonical_repo: Option<PathBuf>,
    subject: Option<String>,
}

impl<'a> Cited<'a> {
    fn new(record: &Path, repo: Option<&'a Path>, subject: Option<String>) -> Self {
        let dir = record.parent().unwrap_or(Path::new(".")).to_path_buf();
        let canonical_dir = std::fs::canonicalize(&dir).ok();
        let canonical_repo = repo.and_then(|path| std::fs::canonicalize(path).ok());
        Self {
            dir,
            canonical_dir,
            repo,
            canonical_repo,
            subject,
        }
    }

    fn relative_path(candidate: &Path) -> Option<PathBuf> {
        let mut has_normal = false;
        for component in candidate.components() {
            match component {
                Component::Normal(_) => has_normal = true,
                Component::CurDir => {}
                Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
            }
        }
        has_normal.then(|| candidate.to_path_buf())
    }

    fn relative(path: &str) -> Option<PathBuf> {
        Self::relative_path(Path::new(path))
    }

    fn absolute_relative(&self, path: &Path) -> Result<PathBuf, &'static str> {
        let Some(repo) = self.repo else {
            return Err("outside repo root");
        };
        let Some(relative) = path.strip_prefix(repo).ok().and_then(Self::relative_path) else {
            return Err("outside repo root");
        };

        let Some(base) = self.canonical_repo.as_deref() else {
            return Err("unsafe path");
        };
        let mut existing = path;
        loop {
            if std::fs::symlink_metadata(existing).is_ok() {
                let Ok(canonical) = std::fs::canonicalize(existing) else {
                    return Err("unsafe path");
                };
                if !canonical.starts_with(base) {
                    return Err("unsafe path");
                }
                break;
            }
            let Some(parent) = existing.parent() else {
                return Err("unsafe path");
            };
            if parent == existing {
                return Err("unsafe path");
            }
            existing = parent;
        }

        Ok(relative)
    }

    fn at_subject(&self, relative: &Path) -> Result<Vec<u8>, &'static str> {
        let (Some(repo), Some(subject)) = (self.repo, self.subject.as_deref()) else {
            return Err("no subject");
        };
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["cat-file", "blob"])
            .arg(format!("{subject}:{}", relative.display()))
            .output()
            .map_err(|_| "missing at subject")?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err("missing at subject")
        }
    }

    fn bytes(&self, path: &str) -> Result<Vec<u8>, &'static str> {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            let relative = self.absolute_relative(candidate)?;
            return self.at_subject(&relative);
        }

        let Some(relative) = Self::relative(path) else {
            return Err("unsafe path");
        };

        let beside = self.dir.join(&relative);
        if beside.is_file() {
            let Some(base) = self.canonical_dir.as_deref() else {
                return Err("unsafe path");
            };
            let Ok(canonical) = std::fs::canonicalize(&beside) else {
                return Err("unsafe path");
            };
            if !canonical.starts_with(base) {
                return Err("unsafe path");
            }
            if let Ok(bytes) = std::fs::read(canonical) {
                return Ok(bytes);
            }
        }

        self.at_subject(&relative)
    }
}

fn subject(v: &Value) -> Option<String> {
    [
        v.get("artifact_identity")
            .and_then(|x| x.get("snapshot_tree")),
        v.get("artifact_identity").and_then(|x| x.get("revision")),
        v.get("subject").and_then(|x| x.get("tree")),
        v.get("subject").and_then(|x| x.get("revision")),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .find(|s| is_hex(s, 40, false))
    .map(str::to_owned)
}

const FILE_DIGEST_MAPS: &[&str] = &[
    "source_files_sha256",
    "fixture_inputs_sha256",
    "generated_inputs_sha256",
    "inputs_sha256",
];

fn check_citation(
    path: &str,
    digest: &str,
    cited: &Cited<'_>,
    out: &mut BTreeSet<&'static str>,
    notes: &mut Vec<String>,
) {
    match cited.bytes(path) {
        Ok(bytes) if digest.trim_start_matches("sha256:") != sha256_hex(&bytes) => {
            out.insert("digest.mismatch");
        }
        Ok(_) => {}
        Err(reason) => notes.push(format!("citation.unchecked: {path} ({reason})")),
    }
}

fn check_pair(
    map: &serde_json::Map<String, Value>,
    path_key: &str,
    digest_key: &str,
    cited: &Cited<'_>,
    out: &mut BTreeSet<&'static str>,
    notes: &mut Vec<String>,
) {
    if let (Some(Value::String(path)), Some(Value::String(digest))) =
        (map.get(path_key), map.get(digest_key))
    {
        check_citation(path, digest, cited, out, notes);
    }
}

fn citations(
    map: &serde_json::Map<String, Value>,
    cited: &Cited<'_>,
    out: &mut BTreeSet<&'static str>,
    notes: &mut Vec<String>,
) {
    check_pair(map, "path", "sha256", cited, out, notes);
    check_pair(map, "source", "source_sha256", cited, out, notes);

    for path_key in map.keys() {
        let Some(prefix) = path_key.strip_suffix("_path") else {
            continue;
        };
        if prefix.is_empty() {
            continue;
        }
        for suffix in ["_sha256", "_digest"] {
            let digest_key = format!("{prefix}{suffix}");
            check_pair(map, path_key, &digest_key, cited, out, notes);
        }
    }

    for map_key in FILE_DIGEST_MAPS {
        let Some(Value::Object(files)) = map.get(*map_key) else {
            continue;
        };
        for (path, digest) in files {
            if let Value::String(digest) = digest {
                if !digest_ok(digest, false) {
                    out.insert("digest.malformed");
                }
                check_citation(path, digest, cited, out, notes);
            }
        }
    }
}

fn walk(
    v: &Value,
    cited: &Cited<'_>,
    strict_time: bool,
    in_sources: bool,
    out: &mut BTreeSet<&'static str>,
    notes: &mut Vec<String>,
) {
    match v {
        Value::Object(map) => {
            citations(map, cited, out, notes);
            for (k, x) in map {
                match x {
                    Value::String(s) if is_digest_key(k) && !digest_ok(s, in_sources) => {
                        out.insert("digest.malformed");
                    }
                    Value::String(s)
                        if is_revision_key(k) && s != "unknown" && !is_hex(s, 40, false) =>
                    {
                        out.insert("revision.malformed");
                    }
                    Value::String(s)
                        if k.ends_with("_at")
                            && (strict_time
                                || s.bytes().next().is_some_and(|b| b.is_ascii_digit()))
                            && !rfc3339(s) =>
                    {
                        out.insert("timestamp.malformed");
                    }
                    Value::Array(items)
                        if k == "parents"
                            && items
                                .iter()
                                .any(|i| i.as_str().is_none_or(|s| !is_hex(s, 40, false))) =>
                    {
                        out.insert("revision.malformed");
                    }
                    _ => {}
                }
                walk(
                    x,
                    cited,
                    strict_time,
                    in_sources || k == "instruction_sources",
                    out,
                    notes,
                );
            }
        }
        Value::Array(items) => {
            for x in items {
                walk(x, cited, strict_time, in_sources, out, notes);
            }
        }
        _ => {}
    }
}

fn blank(v: &Value) -> bool {
    v.is_null() || v.as_str().is_some_and(|s| s.trim().is_empty())
}

fn evidence(v: &Value, l0: &[String], out: &mut BTreeSet<&'static str>) {
    let checks: Vec<&Value> = v["observed_checks"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let mut seen = BTreeSet::new();
    for c in &checks {
        if let Some(id) = c["id"].as_str()
            && !seen.insert(id)
        {
            out.insert("evidence.duplicate_check_id");
        }
    }
    if l0.iter().any(|id| !seen.contains(id.as_str())) {
        out.insert("evidence.missing_required_check");
    }
    for c in &checks {
        let kind = c["kind"].as_str().unwrap_or_default();
        match c["outcome"].as_str().unwrap_or_default() {
            "passed" => {
                if c["exit_status"] != 0 {
                    out.insert("evidence.passed_nonzero_exit");
                }
                if kind == "test"
                    && c["selection_counts"]["tests"]
                        .as_u64()
                        .is_none_or(|n| n == 0)
                {
                    out.insert("evidence.passed_invalid_kind");
                }
            }
            "failed" if blank(&c["error_class"]) => {
                out.insert("evidence.failed_without_error_class");
            }
            "not_run" if blank(&c["reason"]) || !c["exit_status"].is_null() => {
                out.insert("evidence.not_run_without_reason");
            }
            "not_applicable" if blank(&c["policy_rule_id"]) || blank(&c["rationale"]) => {
                out.insert("evidence.not_applicable_without_rule");
            }
            "inconclusive" if kind != "comparison" => {
                out.insert("evidence.inconclusive_outside_comparison");
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct LintReport {
    pub kind: String,
    pub reasons: BTreeSet<&'static str>,
    pub notes: Vec<String>,
    pub schema_issues: Vec<record_schema::Issue>,
}

fn issue(code: &'static str, path: &str, message: impl Into<String>) -> record_schema::Issue {
    record_schema::Issue {
        code,
        path: path.to_string(),
        message: message.into(),
    }
}

fn normalize(report: &mut LintReport) {
    report.notes.sort();
    report.notes.dedup();
    report.schema_issues.sort_by(|left, right| {
        (&left.code, &left.path, &left.message).cmp(&(&right.code, &right.path, &right.message))
    });
    report.schema_issues.dedup_by(|left, right| {
        left.code == right.code && left.path == right.path && left.message == right.message
    });
    for schema_issue in &report.schema_issues {
        report.reasons.insert(schema_issue.code);
    }
}

fn is_json_family(kind: &str) -> bool {
    matches!(
        kind,
        "evidence" | "h4" | "markdown_corpus" | "h5_conformance"
    )
}

fn parse_record(text: &str, record: &Path, kind: Option<&str>) -> Option<Value> {
    let extension = record.extension().and_then(|x| x.to_str());
    let json = match kind {
        Some(kind) => is_json_family(kind),
        None if extension == Some("json") => true,
        None if extension == Some("toml") => false,
        None => return None,
    };
    if json {
        serde_json::from_str(text).ok()
    } else {
        toml::from_str::<toml::Value>(text)
            .ok()
            .and_then(|value| serde_json::to_value(value).ok())
    }
}

fn infer_json(value: &Value) -> (String, Option<&str>, Vec<record_schema::Issue>) {
    let Some(object) = value.as_object() else {
        return (
            "unknown".to_string(),
            None,
            vec![issue(
                "schema.wrong_type",
                "",
                "a record must have an object root",
            )],
        );
    };

    if let Some(record_kind) = object.get("record_kind") {
        let Some(record_kind) = record_kind.as_str() else {
            return (
                "unknown".to_string(),
                None,
                vec![issue(
                    "schema.wrong_type",
                    "/record_kind",
                    "record_kind must be a string",
                )],
            );
        };
        if record_kind != "evidence" {
            return (
                record_kind.to_string(),
                None,
                vec![issue(
                    "schema.invalid_value",
                    "/record_kind",
                    "record_kind must be evidence",
                )],
            );
        }
        return ("evidence".to_string(), Some("evidence"), Vec::new());
    }

    let Some(kind) = object.get("kind") else {
        return (
            "unknown".to_string(),
            None,
            vec![issue(
                "schema.missing_field",
                "",
                "record discriminator is missing",
            )],
        );
    };
    let Some(kind) = kind.as_str() else {
        return (
            "unknown".to_string(),
            None,
            vec![issue("schema.wrong_type", "/kind", "kind must be a string")],
        );
    };
    match kind {
        "h4" | "markdown_corpus" | "h5_conformance" => (kind.to_string(), Some(kind), Vec::new()),
        _ => (
            kind.to_string(),
            None,
            vec![issue(
                "schema.invalid_value",
                "/kind",
                "kind is not a supported record family",
            )],
        ),
    }
}

fn infer_toml(value: &Value) -> String {
    let Some(object) = value.as_object() else {
        return "task".to_string();
    };
    if object.contains_key("acceptor") {
        "acceptance".to_string()
    } else if object.contains_key("authority") || object.contains_key("lanes") {
        "policy".to_string()
    } else {
        "task".to_string()
    }
}

fn family_is_corpus(kind: &str) -> bool {
    matches!(kind, "h4" | "markdown_corpus" | "h5_conformance")
}

/// Produces the structured lint result for a record.
#[must_use]
pub fn report_in(
    record: &Path,
    kind: Option<&str>,
    l0: &[String],
    repo: Option<&Path>,
) -> LintReport {
    let mut report = LintReport {
        kind: kind.unwrap_or("unknown").to_string(),
        reasons: BTreeSet::new(),
        notes: Vec::new(),
        schema_issues: Vec::new(),
    };

    let Ok(text) = std::fs::read_to_string(record) else {
        report
            .schema_issues
            .push(issue("schema.wrong_type", "", "record could not be read"));
        normalize(&mut report);
        return report;
    };
    let Some(value) = parse_record(&text, record, kind) else {
        report
            .schema_issues
            .push(issue("schema.wrong_type", "", "record could not be parsed"));
        normalize(&mut report);
        return report;
    };

    let (family, discriminator_issues) = if let Some(kind) = kind {
        (kind.to_string(), Vec::new())
    } else if record.extension().and_then(|x| x.to_str()) == Some("json") {
        let (kind, family, issues) = infer_json(&value);
        (family.map_or(kind, str::to_owned), issues)
    } else {
        (infer_toml(&value), Vec::new())
    };
    report.kind = family.clone();
    report.schema_issues.extend(discriminator_issues);

    let supported = matches!(
        family.as_str(),
        "task" | "acceptance" | "policy" | "evidence" | "h4" | "markdown_corpus" | "h5_conformance"
    );
    if supported && report.schema_issues.is_empty() {
        report
            .schema_issues
            .extend(record_schema::validate(&value, &family));
    } else if !supported && report.schema_issues.is_empty() {
        report.schema_issues.push(issue(
            "schema.invalid_value",
            "",
            format!("unsupported record schema family {family}"),
        ));
    }

    if supported {
        let subject = subject(&value);
        let cited = Cited::new(record, repo, subject);
        let strict_time = family == "evidence" || family_is_corpus(&family);
        walk(
            &value,
            &cited,
            strict_time,
            false,
            &mut report.reasons,
            &mut report.notes,
        );
        if family == "evidence" {
            evidence(&value, l0, &mut report.reasons);
        }
    }

    normalize(&mut report);
    report
}

/// The reason codes for one explicitly declared record family.
#[must_use]
pub fn lint(record: &Path, kind: &str, l0: &[String]) -> BTreeSet<&'static str> {
    lint_in(record, kind, l0, None)
}

/// As [`lint`], resolving cited files at their historical subject when needed.
#[must_use]
pub fn lint_in(
    record: &Path,
    kind: &str,
    l0: &[String],
    repo: Option<&Path>,
) -> BTreeSet<&'static str> {
    report_in(record, Some(kind), l0, repo).reasons
}

/// The L0 check ids of this repository's policy.
///
/// # Errors
/// If the policy cannot be loaded.
pub fn l0_ids(root: &Path) -> crate::error::Result<Vec<String>> {
    let loaded = crate::policy::load(root)?;
    Ok(loaded
        .policy
        .lanes
        .get("L0")
        .map(|lane| lane.checks.iter().map(|check| check.id.clone()).collect())
        .unwrap_or_default())
}

/// Lints each requested file, printing its reason codes; exit 1 if any record
/// is rejected.
///
/// # Errors
/// If the registered schemas or policy cannot be loaded.
pub fn run(root: &Path, files: &[PathBuf]) -> crate::error::Result<u8> {
    if let Some(code) = record_schema::configuration_error() {
        return Err(crate::error::Error::new(format!(
            "record schema configuration failed: {code}"
        )));
    }
    let l0 = l0_ids(root)?;
    let mut rejected = false;
    for file in files {
        let report = report_in(file, None, &l0, Some(root));
        if report.reasons.is_empty() {
            if report.notes.is_empty() {
                println!("{}: accepted ({})", file.display(), report.kind);
            } else {
                println!(
                    "{}: accepted with unchecked citations ({})",
                    file.display(),
                    report.kind
                );
            }
        } else {
            rejected = true;
            println!(
                "{}: rejected ({}): {}",
                file.display(),
                report.kind,
                report
                    .reasons
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            for schema_issue in &report.schema_issues {
                println!(
                    "  schema {}: {}",
                    if schema_issue.path.is_empty() {
                        "/"
                    } else {
                        &schema_issue.path
                    },
                    schema_issue.message
                );
            }
        }
        for note in &report.notes {
            println!("  {note}");
        }
    }
    Ok(u8::from(rejected))
}

#[cfg(test)]
mod tests {
    use super::rfc3339;

    #[test]
    fn calendar_offsets_and_bytes() {
        assert!(rfc3339("2026-09-22T12:00:00Z"));
        assert!(rfc3339("2024-02-29T00:00:00.123+05:30"));
        assert!(!rfc3339("2026-02-30T12:00:00Z"));
        assert!(!rfc3339("2025-02-29T12:00:00Z"));
        assert!(!rfc3339("2026-09-22T12:00:00+25:00"));
        assert!(!rfc3339("2026-09-22T12:00:00\u{e9}xxxx"));
        assert!(!rfc3339("not-a-timestamp"));
    }
}
