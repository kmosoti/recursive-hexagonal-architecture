//! Obligations (contract §1.1), predicates (§1.2), exceptions and
//! MergeAllowed (§1.3).

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{glob, parse_rfc3339};

fn strs(v: &Value) -> Vec<&str> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// A check: id, kind, params.
#[derive(Debug, Clone, PartialEq)]
struct Check {
    id: String,
    kind: String,
    params: Map<String, Value>,
}

impl Check {
    fn from(v: &Value) -> Option<Self> {
        Some(Self {
            id: v.get("id")?.as_str()?.to_owned(),
            kind: v
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            params: v
                .get("params")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
        })
    }
    fn to_json(&self) -> Value {
        json!({"id": self.id, "kind": self.kind, "params": Value::Object(self.params.clone())})
    }
}

enum Order {
    Smaller,
    Larger,
    Superset,
    Equal,
}

fn order(key: &str) -> Order {
    match key {
        "delta" => Order::Smaller,
        "n" | "executions" | "proptest_cases" => Order::Larger,
        "selection" => Order::Superset,
        _ => Order::Equal,
    }
}

fn superset(a: &Value, b: &Value) -> Option<bool> {
    let (a, b) = (a.as_array()?, b.as_array()?);
    Some(b.iter().all(|x| a.contains(x)))
}

/// Whether `a` is at least as strict as `b` for `key`.
fn at_least_as_strict(key: &str, a: &Value, b: &Value) -> bool {
    match order(key) {
        Order::Smaller => matches!((a.as_f64(), b.as_f64()), (Some(x), Some(y)) if x <= y),
        Order::Larger => matches!((a.as_f64(), b.as_f64()), (Some(x), Some(y)) if x >= y),
        Order::Superset => superset(a, b) == Some(true),
        Order::Equal => a == b,
    }
}

/// The stricter of two values for `key`, or `None` when incomparable.
fn stricter(key: &str, a: &Value, b: &Value) -> Option<Value> {
    if at_least_as_strict(key, a, b) {
        Some(a.clone())
    } else if at_least_as_strict(key, b, a) {
        Some(b.clone())
    } else {
        None
    }
}

/// Joins two checks with one id; `None` is a conflict.
fn join(a: &Check, b: &Check) -> Option<Check> {
    if a.kind != b.kind {
        return None;
    }
    let mut params = a.params.clone();
    for (k, v) in &b.params {
        let joined = match params.get(k) {
            Some(existing) => stricter(k, existing, v)?,
            None => v.clone(),
        };
        params.insert(k.clone(), joined);
    }
    Some(Check {
        id: a.id.clone(),
        kind: a.kind.clone(),
        params,
    })
}

/// `params(check) ⊑ params(entry)`: every key of the check is present in the
/// entry and the entry's value is at least as strict.
fn refines(check: &Check, entry: &Value) -> bool {
    let empty = Map::new();
    let got = entry
        .get("params")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    check
        .params
        .iter()
        .all(|(k, v)| got.get(k).is_some_and(|e| at_least_as_strict(k, e, v)))
}

/// `R_eff`, or `None` on a conflict.
fn obligations(f: &Value) -> Option<Vec<Check>> {
    let policy = &f["policy"];
    let surface = strs(&f["surface"]);
    let root_checks: BTreeMap<String, Check> = policy["checks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Check::from)
        .map(|c| (c.id.clone(), c))
        .collect();
    let rules = policy["rules"].as_array().cloned().unwrap_or_default();
    let mut required: Vec<String> = Vec::new();
    let mut any_unmatched = false;
    for path in &surface {
        let mut matched = false;
        for rule in &rules {
            if rule["scope"].as_str().is_some_and(|s| glob(s, path)) {
                matched = true;
                required.extend(strs(&rule["requires"]).into_iter().map(str::to_owned));
            }
        }
        any_unmatched |= !matched;
    }
    if any_unmatched {
        required.extend(
            strs(&policy["default_obligations"])
                .into_iter()
                .map(str::to_owned),
        );
    }
    let locals: Vec<&Value> = f["local_policies"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| {
            l["scope"]
                .as_str()
                .is_some_and(|s| surface.iter().any(|p| glob(s, p)))
        })
        .collect();
    for local in &locals {
        for rule in local["rules"].as_array().into_iter().flatten() {
            if rule["scope"]
                .as_str()
                .is_some_and(|s| surface.iter().any(|p| glob(s, p)))
            {
                required.extend(strs(&rule["requires"]).into_iter().map(str::to_owned));
            }
        }
    }
    required.sort();
    required.dedup();
    let mut out = Vec::new();
    for id in required {
        let mut check: Option<Check> = root_checks.get(&id).cloned();
        for local in &locals {
            for c in local["checks"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Check::from)
                .filter(|c| c.id == id)
            {
                check = Some(match check {
                    Some(existing) => join(&existing, &c)?,
                    None => c,
                });
            }
        }
        out.push(check.unwrap_or(Check {
            id,
            kind: String::new(),
            params: Map::new(),
        }));
    }
    Some(out)
}

fn non_empty(v: &Value) -> bool {
    v.as_str().is_some_and(|s| !s.trim().is_empty())
}

/// The decision for one fixture.
#[must_use]
pub fn evaluate(f: &Value) -> Value {
    let policy = &f["policy"];
    let evidence = &f["evidence"];
    let exception = &f["exception"];
    let has_exception = !exception.is_null();
    let acceptor_ok =
        strs(&policy["acceptance_authority"]).contains(&f["acceptor"].as_str().unwrap_or_default());
    let Some(r_eff) = obligations(f) else {
        return json!({
            "r_eff": Value::Null, "conflict": true,
            "authentic": false, "applicable": false, "complete": false, "passed": false,
            "eligible": false, "valid_exception": if has_exception { json!(false) } else { Value::Null },
            "merge_allowed": false,
        });
    };
    let authentic = strs(&policy["trusted_producers"])
        .contains(&evidence["producer"].as_str().unwrap_or_default())
        && evidence["integrity"] == "valid";
    let applicable = evidence["subject"] == f["candidate_tree"]
        && evidence["policy"] == policy["digest"]
        && evidence["base"] == f["base"]
        && strs(&policy["required_inputs"]).iter().all(|name| {
            evidence["inputs"]
                .get(*name)
                .and_then(Value::as_str)
                .is_some_and(|v| !v.is_empty() && v != "unknown")
        });
    let entries: Vec<&Value> = evidence["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let count = |id: &str| entries.iter().filter(|e| e["id"] == id).count();
    let mut ids: Vec<&str> = entries.iter().filter_map(|e| e["id"].as_str()).collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    let complete = ids.len() == total
        && r_eff.iter().all(|c| {
            count(&c.id) == 1
                && entries
                    .iter()
                    .any(|e| e["id"] == c.id.as_str() && refines(c, e))
        });
    let passed_one = |c: &Check| -> bool {
        let Some(e) = entries.iter().find(|e| e["id"] == c.id.as_str()) else {
            return false;
        };
        e["outcome"] == "passed"
            && match c.kind.as_str() {
                "test" => e["selected_tests"].as_u64().is_some_and(|n| n > 0),
                "comparison" => e["performed"] == true,
                "corpus" => {
                    !c.params.get("corpus_digest").is_none_or(Value::is_null)
                        && e["corpus_digest"] == c.params["corpus_digest"]
                }
                _ => true,
            }
    };
    let passed = r_eff.iter().all(passed_one);
    let eligible = authentic && applicable && complete && passed;
    let valid_exception = if has_exception {
        let x = exception;
        let failed: Vec<&str> = r_eff
            .iter()
            .filter(|c| !passed_one(c))
            .map(|c| c.id.as_str())
            .collect();
        let waived = strs(&x["waived"]);
        let non_waivable = strs(&policy["non_waivable"]);
        let now = f["now"].as_str().and_then(parse_rfc3339);
        let issued = x["issued_at"].as_str().and_then(parse_rfc3339);
        let expires = x["expires_at"].as_str().and_then(parse_rfc3339);
        let in_window =
            matches!((issued, now, expires), (Some(i), Some(n), Some(e)) if i <= n && n < e);
        let issuer = x["issuer"].as_str().unwrap_or_default();
        let self_issued = issuer
            == f["accountable_change_authority"]
                .as_str()
                .unwrap_or_default();
        let solo_ok = policy["profile"] == "ECC-Solo"
            && match (
                x["logged_at"].as_str().and_then(parse_rfc3339),
                now,
                policy["cooling_off_hours"].as_i64(),
            ) {
                (Some(logged), Some(n), Some(h)) => n >= logged + h * 3600,
                _ => false,
            };
        json!(
            authentic
                && applicable
                && complete
                && x["authentic"] == true
                && x["subject"] == f["candidate_tree"]
                && x["base"] == f["base"]
                && x["policy"] == policy["digest"]
                && strs(&policy["exception_authority"]).contains(&issuer)
                && failed.iter().all(|c| waived.contains(c))
                && !waived.iter().any(|w| non_waivable.contains(w))
                && in_window
                && x["revoked"] != true
                && non_empty(&x["reason"])
                && non_empty(&x["compensating_control"])
                && non_empty(&x["follow_up"])
                && (!self_issued || solo_ok)
        )
    } else {
        Value::Null
    };
    let merge_allowed = acceptor_ok && (eligible || valid_exception == true);
    json!({
        "r_eff": r_eff.iter().map(Check::to_json).collect::<Vec<_>>(), "conflict": false,
        "authentic": authentic, "applicable": applicable, "complete": complete, "passed": passed,
        "eligible": eligible, "valid_exception": valid_exception, "merge_allowed": merge_allowed,
    })
}
