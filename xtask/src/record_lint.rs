//! `cargo xtask rha lint`: the record lint of P-B stage 2, graded by the
//! registered record corpus. Reason codes are exactly those of
//! `docs/architecture/verifier-contract.md` §2, fixed before this code.
//!
//! Field rules (choices the contract leaves open, applied uniformly):
//! * a **digest** field is a key containing `sha256` or ending in `digest`
//!   (not `digest_algorithm`); valid values are `sha256:` + 64 lowercase hex
//!   or a bare 64 lowercase hex. `unknown` is accepted only inside
//!   `instruction_sources`, whose message entries have no file to hash (spec
//!   §11.6.5); anywhere else, such as a policy digest, it is malformed
//!   (corpus R025);
//! * a **revision** field is `revision`, `tree`, `commit`, `git_rev`, a key
//!   ending in `_revision`, `_tree` or `_git_rev`, or an item of `parents`;
//!   valid values are 40 hex, `null`, or `unknown`;
//! * a **timestamp** field is a key ending in `_at`. In evidence records,
//!   which tools write, every such field is checked (corpus R038); in
//!   hand-written task and acceptance records only a value beginning with a
//!   digit is, because prose such as `found_at = "review, 2026-09-20"` occurs
//!   there. It must be RFC 3339 with a valid date, time and `Z` or `±HH:MM`
//!   offset (corpus R037: `+25:00` is not one);
//! * a **cited file** is an object with `path` and `sha256`; when the file
//!   exists beside the record, its bytes must hash to the cited digest.
//!   Otherwise, when git has the record's subject (`snapshot_tree`, else
//!   `artifact_identity.revision`) and the path in it, the bytes at that
//!   subject are hashed instead, because repository records cite paths from
//!   the repository root (Opus 5.5 review of pull request 17, finding 3). A
//!   subject git does not have, such as the synthetic corpus's, leaves the
//!   contract's beside-the-record rule as the only check.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

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
/// Byte-based throughout, so a non-ASCII suffix is malformed rather than a
/// panic, and 30 February is rejected (review round 1, findings 5 and 6).
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

/// Where a cited file's bytes come from: beside the record, else the
/// record's subject in git.
struct Cited<'a> {
    dir: &'a Path,
    repo: Option<&'a Path>,
    subject: Option<String>,
}

impl Cited<'_> {
    fn bytes(&self, path: &str) -> Option<Vec<u8>> {
        if path.starts_with('/') || path.contains("..") {
            return None;
        }
        let beside = self.dir.join(path);
        if beside.is_file() {
            return std::fs::read(beside).ok();
        }
        let (repo, subject) = (self.repo?, self.subject.as_deref()?);
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["cat-file", "blob", &format!("{subject}:{path}")])
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }
}

fn walk(
    v: &Value,
    cited: &Cited<'_>,
    strict_time: bool,
    in_sources: bool,
    out: &mut BTreeSet<&'static str>,
) {
    match v {
        Value::Object(map) => {
            if let (Some(Value::String(path)), Some(Value::String(digest))) =
                (map.get("path"), map.get("sha256"))
            {
                if let Some(bytes) = cited.bytes(path)
                    && digest.trim_start_matches("sha256:") != sha256_hex(&bytes)
                {
                    out.insert("digest.mismatch");
                }
            }
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
                );
            }
        }
        Value::Array(items) => {
            for x in items {
                walk(x, cited, strict_time, in_sources, out);
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

/// The reason codes for one record; empty means accepted. `kind` is
/// `evidence` (JSON), `task` or `acceptance` (TOML); `l0` is the policy's
/// required L0 check ids.
#[must_use]
pub fn lint(record: &Path, kind: &str, l0: &[String]) -> BTreeSet<&'static str> {
    lint_in(record, kind, l0, None)
}

/// As [`lint`], resolving cited files that are not beside the record in the
/// git repository at `repo`, at the record's subject.
#[must_use]
pub fn lint_in(
    record: &Path,
    kind: &str,
    l0: &[String],
    repo: Option<&Path>,
) -> BTreeSet<&'static str> {
    let mut out = BTreeSet::new();
    let Ok(text) = std::fs::read_to_string(record) else {
        out.insert("schema.wrong_type");
        return out;
    };
    let value: Option<Value> = if kind == "evidence" {
        serde_json::from_str(&text).ok()
    } else {
        toml::from_str::<toml::Value>(&text)
            .ok()
            .and_then(|t| serde_json::to_value(t).ok())
    };
    let Some(value) = value else {
        out.insert("schema.wrong_type");
        return out;
    };
    let identity = &value["artifact_identity"];
    let subject = [&identity["snapshot_tree"], &identity["revision"]]
        .into_iter()
        .filter_map(Value::as_str)
        .find(|s| is_hex(s, 40, false))
        .map(str::to_owned);
    let cited = Cited {
        dir: record.parent().unwrap_or(Path::new(".")),
        repo,
        subject,
    };
    walk(&value, &cited, kind == "evidence", false, &mut out);
    if kind == "evidence" {
        evidence(&value, l0, &mut out);
    }
    out
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
        .map(|l| l.checks.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default())
}

/// The command: lints each file, printing its reason codes; exit 1 if any
/// record is rejected.
///
/// # Errors
/// If the policy cannot be loaded.
pub fn run(root: &Path, files: &[std::path::PathBuf]) -> crate::error::Result<u8> {
    let l0 = l0_ids(root)?;
    let mut rejected = 0;
    for f in files {
        let kind = if f.extension().is_some_and(|e| e == "json") {
            "evidence"
        } else if std::fs::read_to_string(f).is_ok_and(|t| t.contains("[acceptor]")) {
            "acceptance"
        } else {
            "task"
        };
        let reasons = lint_in(f, kind, &l0, Some(root));
        if reasons.is_empty() {
            println!("{}: accepted ({kind})", f.display());
        } else {
            rejected += 1;
            println!(
                "{}: rejected ({kind}): {}",
                f.display(),
                reasons.into_iter().collect::<Vec<_>>().join(", ")
            );
        }
    }
    Ok(u8::from(rejected > 0))
}

#[cfg(test)]
mod tests {
    use super::rfc3339;

    #[test]
    fn calendar_offsets_and_bytes() {
        assert!(rfc3339("2026-09-22T12:00:00Z"));
        assert!(rfc3339("2024-02-29T00:00:00.123+05:30"));
        assert!(!rfc3339("2026-02-30T12:00:00Z"), "30 February");
        assert!(!rfc3339("2025-02-29T12:00:00Z"), "not a leap year");
        assert!(!rfc3339("2026-09-22T12:00:00+25:00"));
        assert!(
            !rfc3339("2026-09-22T12:00:00\u{e9}xxxx"),
            "a non-ASCII suffix is malformed, not a panic"
        );
        assert!(!rfc3339("not-a-timestamp"));
    }
}
