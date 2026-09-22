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
        let s = v.get(r)?;
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())
            .flatten()
    };
    let (Some(_), Some(mo), Some(d), Some(h), Some(mi), Some(se)) = (
        num(0..4),
        num(5..7),
        num(8..10),
        num(11..13),
        num(14..16),
        num(17..19),
    ) else {
        return false;
    };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return false;
    }
    let mut rest = &v[19..];
    if let Some(frac) = rest.strip_prefix('.') {
        let n = frac.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return false;
        }
        rest = &frac[n..];
    }
    if matches!(rest, "Z" | "z") {
        return true;
    }
    rest.len() == 6
        && matches!(&rest[..1], "+" | "-")
        && &rest[3..4] == ":"
        && rest[1..3].parse::<u32>().is_ok_and(|oh| oh <= 23)
        && rest[4..6].parse::<u32>().is_ok_and(|om| om <= 59)
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

fn walk(
    v: &Value,
    dir: &Path,
    strict_time: bool,
    in_sources: bool,
    out: &mut BTreeSet<&'static str>,
) {
    match v {
        Value::Object(map) => {
            if let (Some(Value::String(path)), Some(Value::String(cited))) =
                (map.get("path"), map.get("sha256"))
            {
                let file = dir.join(path);
                if !path.starts_with('/') && !path.contains("..") && file.is_file() {
                    let actual = std::fs::read(&file)
                        .map(|b| sha256_hex(&b))
                        .unwrap_or_default();
                    if cited.trim_start_matches("sha256:") != actual {
                        out.insert("digest.mismatch");
                    }
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
                    dir,
                    strict_time,
                    in_sources || k == "instruction_sources",
                    out,
                );
            }
        }
        Value::Array(items) => {
            for x in items {
                walk(x, dir, strict_time, in_sources, out);
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
    let dir = record.parent().unwrap_or(Path::new("."));
    walk(&value, dir, kind == "evidence", false, &mut out);
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
        let reasons = lint(f, kind, &l0);
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
