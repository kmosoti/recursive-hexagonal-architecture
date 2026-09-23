#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
// Written independently from the contract and spec, without reading the corpus or the implementation.

use serde_json::{Value, json};
use std::collections::BTreeSet;

pub fn evaluate(fixture: &Value) -> Value {
    let exception = &fixture["exception"];
    let Some(checks) = obligations(fixture) else {
        return json!({
            "r_eff": null,
            "conflict": true,
            "authentic": false,
            "applicable": false,
            "complete": false,
            "passed": false,
            "eligible": false,
            "valid_exception": if exception.is_null() { Value::Null } else { json!(false) },
            "merge_allowed": false,
        });
    };

    let policy = &fixture["policy"];
    let evidence = &fixture["evidence"];
    let entries = array(evidence, "entries");
    let authentic = array(policy, "trusted_producers").contains(&evidence["producer"])
        && evidence["integrity"] == "valid";
    let applicable = evidence["subject"] == fixture["candidate_tree"]
        && evidence["policy"] == policy["digest"]
        && evidence["base"] == fixture["base"]
        && array(policy, "required_inputs").iter().all(|name| {
            name.as_str()
                .and_then(|name| evidence["inputs"].get(name))
                .is_some_and(known_input)
        });

    let mut seen = BTreeSet::new();
    let unique_ids = entries
        .iter()
        .all(|entry| entry["id"].as_str().is_some_and(|id| seen.insert(id)));
    let complete = unique_ids
        && checks.iter().all(|check| {
            let matching: Vec<_> = entries
                .iter()
                .filter(|entry| entry["id"] == check["id"])
                .collect();
            matching.len() == 1 && params_refine(&check["params"], &matching[0]["params"])
        });
    let failed: Vec<_> = checks
        .iter()
        .filter(|check| !check_passed(check, entries))
        .collect();
    let passed = failed.is_empty();
    let eligible = authentic && applicable && complete && passed;
    let valid_exception = if exception.is_null() {
        Value::Null
    } else {
        json!(authentic && applicable && complete && exception_valid(fixture, exception, &failed))
    };
    let merge_allowed = array(policy, "acceptance_authority").contains(&fixture["acceptor"])
        && (eligible || valid_exception == true);

    json!({
        "r_eff": checks,
        "conflict": false,
        "authentic": authentic,
        "applicable": applicable,
        "complete": complete,
        "passed": passed,
        "eligible": eligible,
        "valid_exception": valid_exception,
        "merge_allowed": merge_allowed,
    })
}

fn array<'a>(object: &'a Value, key: &str) -> &'a [Value] {
    object[key].as_array().map_or(&[], Vec::as_slice)
}

fn known_input(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.is_empty() && text != "unknown",
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        _ => true,
    }
}

// Work from required ids first: unused check declarations are not obligations.
fn obligations(fixture: &Value) -> Option<Vec<Value>> {
    let policy = &fixture["policy"];
    let surface = array(fixture, "surface");
    let mut required = BTreeSet::new();
    for path in surface {
        let mut classified = false;
        for rule in array(policy, "rules") {
            if scope_matches(&rule["scope"], path) {
                classified = true;
                for id in array(rule, "requires") {
                    required.insert(id.as_str()?);
                }
            }
        }
        if !classified {
            for id in array(policy, "default_obligations") {
                required.insert(id.as_str()?);
            }
        }
    }

    let mut declarations: Vec<_> = array(policy, "checks").iter().collect();
    for local in array(fixture, "local_policies") {
        if !surface
            .iter()
            .any(|path| scope_matches(&local["scope"], path))
        {
            continue;
        }
        declarations.extend(array(local, "checks"));
        for rule in array(local, "rules") {
            if surface
                .iter()
                .any(|path| scope_matches(&rule["scope"], path))
            {
                for id in array(rule, "requires") {
                    required.insert(id.as_str()?);
                }
            }
        }
    }

    let mut result = Vec::new();
    for id in required {
        let mut matching = declarations
            .iter()
            .copied()
            .filter(|check| check["id"] == id);
        // An obligation without a declaration cannot yield an effective check.
        let first = matching.next()?;
        let mut params = first["params"].as_object()?.clone();
        for other in matching {
            if first["kind"] != other["kind"] {
                return None;
            }
            for (key, value) in other["params"].as_object()? {
                match params.get(key) {
                    None => {
                        params.insert(key.clone(), value.clone());
                    }
                    Some(old) if parameter_refines(key, value, old) => {}
                    Some(old) if parameter_refines(key, old, value) => {
                        params.insert(key.clone(), value.clone());
                    }
                    Some(_) => return None,
                }
            }
        }
        result.push(json!({"id": id, "kind": first["kind"], "params": params}));
    }
    Some(result)
}

fn scope_matches(scope: &Value, path: &Value) -> bool {
    match (scope.as_str(), path.as_str()) {
        (Some(scope), Some(path)) => glob_segments(
            &scope.split('/').collect::<Vec<_>>(),
            &path.split('/').collect::<Vec<_>>(),
        ),
        _ => false,
    }
}

fn glob_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => {
            glob_segments(rest, path) || (!path.is_empty() && glob_segments(pattern, &path[1..]))
        }
        Some((segment, rest)) => {
            !path.is_empty()
                && glob_characters(segment.as_bytes(), path[0].as_bytes())
                && glob_segments(rest, &path[1..])
        }
    }
}

fn glob_characters(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((&b'*', rest)) => {
            glob_characters(rest, text)
                || (!text.is_empty() && glob_characters(pattern, &text[1..]))
        }
        Some((&character, rest)) => {
            text.first() == Some(&character) && glob_characters(rest, &text[1..])
        }
    }
}

// required ⊑ actual; extra keys in actual add requirements.
fn params_refine(required: &Value, actual: &Value) -> bool {
    match (required.as_object(), actual.as_object()) {
        (Some(required), Some(actual)) => required.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|other| parameter_refines(key, value, other))
        }),
        _ => false,
    }
}

fn parameter_refines(key: &str, required: &Value, actual: &Value) -> bool {
    match key {
        "delta" => number_le(actual, required),
        "n" | "executions" | "proptest_cases" => number_le(required, actual),
        "selection" => match (required.as_array(), actual.as_array()) {
            (Some(required), Some(actual)) => required.iter().all(|item| actual.contains(item)),
            _ => false,
        },
        // This includes timeout_secs: only equality is ordered.
        _ => required == actual,
    }
}

fn number_le(left: &Value, right: &Value) -> bool {
    // Preserve exact comparisons for integers larger than f64's precision.
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left <= right;
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return left <= right;
    }
    match (left.as_f64(), right.as_f64()) {
        (Some(left), Some(right)) => left <= right,
        _ => false,
    }
}

fn check_passed(check: &Value, entries: &[Value]) -> bool {
    let Some(entry) = entries.iter().find(|entry| entry["id"] == check["id"]) else {
        return false;
    };
    // Duplicate ids are rejected separately by Complete.
    entry["outcome"] == "passed"
        && match check["kind"].as_str() {
            Some("test") => entry["selected_tests"].as_f64().is_some_and(|n| n > 0.0),
            Some("comparison") => entry["performed"] == true,
            Some("corpus") => check["params"]
                .get("corpus_digest")
                .is_some_and(|digest| entry.get("corpus_digest") == Some(digest)),
            _ => true,
        }
}

fn exception_valid(fixture: &Value, exception: &Value, failed: &[&Value]) -> bool {
    let policy = &fixture["policy"];
    let waived = array(exception, "waived");
    if exception["authentic"] != true
        || exception["subject"] != fixture["candidate_tree"]
        || exception["base"] != fixture["base"]
        || exception["policy"] != policy["digest"]
        || !array(policy, "exception_authority").contains(&exception["issuer"])
        || !failed.iter().all(|check| waived.contains(&check["id"]))
        || waived
            .iter()
            .any(|id| array(policy, "non_waivable").contains(id))
        || exception["revoked"] != false
        || ["reason", "compensating_control", "follow_up"]
            .iter()
            .any(|key| {
                !exception[*key]
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
            })
    {
        return false;
    }

    let (Some(now), Some(issued), Some(expires)) = (
        timestamp(&fixture["now"]),
        timestamp(&exception["issued_at"]),
        timestamp(&exception["expires_at"]),
    ) else {
        return false;
    };
    if issued > now || now >= expires {
        return false;
    }
    if exception["issuer"] != fixture["accountable_change_authority"] {
        return true;
    }
    if policy["profile"] != "ECC-Solo" {
        return false;
    }
    let (Some((logged_seconds, fraction)), Some(hours)) = (
        timestamp(&exception["logged_at"]),
        policy["cooling_off_hours"].as_i64(),
    ) else {
        return false;
    };
    if hours < 0 {
        return false;
    }
    hours
        .checked_mul(3600)
        .and_then(|delay| logged_seconds.checked_add(delay))
        .is_some_and(|ready| now >= (ready, fraction))
}

// UTC seconds and fractional digits, with trailing zeroes removed. Tuple ordering
// then compares instants exactly, including fractions of arbitrary precision.
fn timestamp(value: &Value) -> Option<(i64, String)> {
    let text = value.as_str()?;
    let bytes = text.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let digits = |start: usize, end: usize| -> Option<i64> {
        let part = text.get(start..end)?;
        if !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    let year = digits(0, 4)?;
    let month = digits(5, 7)?;
    let day = digits(8, 10)?;
    let hour = digits(11, 13)?;
    let minute = digits(14, 16)?;
    let second = digits(17, 19)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day < 1 || day > month_days || hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    let mut position = 19;
    let mut fraction = String::new();
    if bytes[position] == b'.' {
        position += 1;
        let start = position;
        while bytes.get(position).is_some_and(u8::is_ascii_digit) {
            position += 1;
        }
        if start == position {
            return None;
        }
        fraction = text.get(start..position)?.trim_end_matches('0').to_owned();
    }
    let offset = match bytes.get(position)? {
        b'Z' | b'z' if position + 1 == bytes.len() => 0,
        sign @ (b'+' | b'-') if position + 6 == bytes.len() && bytes[position + 3] == b':' => {
            let hours = digits(position + 1, position + 3)?;
            let minutes = digits(position + 4, position + 6)?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            let magnitude = hours * 3600 + minutes * 60;
            if *sign == b'+' { magnitude } else { -magnitude }
        }
        _ => return None,
    };

    // Count Gregorian days using March-based years and 400-year eras.
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_in_era = year - era * 400;
    let march_month = if month > 2 { month - 3 } else { month + 9 };
    let day_in_year = (153 * march_month + 2) / 5 + day - 1;
    let days = era * 146097 + year_in_era * 365 + year_in_era / 4 - year_in_era / 100 + day_in_year
        - 719468;
    Some((
        days * 86400 + hour * 3600 + minute * 60 + second - offset,
        fraction,
    ))
}
