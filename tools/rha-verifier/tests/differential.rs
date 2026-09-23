//! Differential test (plan §2.4 rule 4): `rha_verifier::evaluate` against an
//! independent reference written by a separate session from the contract
//! alone, without reading the corpus or this crate's source. They must agree
//! on every registered fixture and on random mutations of them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod reference;

use proptest::prelude::*;
use serde_json::{Value, json};

fn corpus() -> Vec<Value> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap())
        .collect()
}

#[test]
fn both_implementations_decide_every_registered_fixture_alike() {
    let mut disagreements = Vec::new();
    for f in corpus() {
        let (a, b) = (rha_verifier::evaluate(&f), reference::evaluate(&f));
        if a != b {
            disagreements.push(format!("{}: ours {a}\n  reference {b}", f["id"]));
        }
    }
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}

/// The element at `pick` modulo the length, if the value is a non-empty array.
fn nth_mut(v: &mut Value, pick: usize) -> Option<&mut Value> {
    let a = v.as_array_mut()?;
    let n = a.len();
    if n == 0 { None } else { a.get_mut(pick % n) }
}

/// A mutation of a fixture: each arm changes one thing a predicate reads.
fn mutate(mut f: Value, which: u8, pick: usize) -> Value {
    let paths = [
        "crates/core/src/lib.rs",
        "docs/a.md",
        ".rha/policy.toml",
        "unmatched/x",
        "crates/core/AGENTS.md",
    ];
    match which % 12 {
        0 => f["surface"] = json!([paths[pick % paths.len()]]),
        1 => {
            if let Some(e) = nth_mut(&mut f["evidence"]["entries"], pick) {
                e["outcome"] = json!(["passed", "failed", "not_run", "inconclusive"][pick % 4]);
            }
        }
        2 => {
            if let Some(e) = nth_mut(&mut f["evidence"]["entries"], pick) {
                e["selected_tests"] = json!(pick % 3);
            }
        }
        3 => {
            if let Some(a) = f["evidence"]["entries"].as_array_mut()
                && let Some(first) = a.first().cloned()
            {
                a.push(first);
            }
        }
        4 => f["evidence"]["integrity"] = json!(["valid", "invalid", "absent"][pick % 3]),
        5 => {
            f["now"] = json!(
                [
                    "2026-01-01T00:00:00Z",
                    "2026-09-22T12:00:00Z",
                    "2030-01-01T00:00:00Z"
                ][pick % 3]
            )
        }
        6 => {
            if f["exception"].is_object() {
                f["exception"]["revoked"] = json!(pick.is_multiple_of(2));
            }
        }
        7 => {
            if let Some(c) = nth_mut(&mut f["policy"]["checks"], pick) {
                c["params"]["timeout_secs"] = json!(300 + 300 * (pick % 3));
            }
        }
        8 => f["acceptor"] = json!(["human:kennedy", "agent:executor"][pick % 2]),
        // The ECC-Solo cooling-off window, within the contract's domain. This
        // arm was missing and fell through to the producer arm (Opus 5.5
        // review of pull request 17, finding 4).
        9 => {
            f["policy"]["cooling_off_hours"] = json!([0, 24, 48][pick % 3]);
            if f["exception"].is_object() {
                f["exception"]["logged_at"] = json!(
                    [
                        "2026-09-20T12:00:00Z",
                        "2026-09-21T12:00:00Z",
                        "2026-09-22T11:00:00Z"
                    ][pick % 3]
                );
            }
        }
        // Fractional times on the exception and at the decision instant
        // (review round 1, finding 1).
        10 if f["exception"].is_object() => {
            let frac = ["", ".100", ".900", ".999999999"][pick % 4];
            for key in ["logged_at", "issued_at"] {
                if let Some(t) = f["exception"][key].as_str().map(str::to_owned) {
                    f["exception"][key] = json!(format!(
                        "{}{frac}Z",
                        t.trim_end_matches('Z')
                            .split('.')
                            .next()
                            .unwrap_or_default()
                    ));
                }
            }
        }
        // Integer parameters around 2^53, on the check and on its entries
        // (review round 1, finding 2).
        11 => {
            let big = 9_007_199_254_740_992_u64 + (pick % 3) as u64;
            if let Some(c) = nth_mut(&mut f["policy"]["checks"], pick) {
                c["params"]["n"] = json!(big + 1);
            }
            if let Some(e) = nth_mut(&mut f["evidence"]["entries"], pick) {
                e["params"]["n"] = json!(big);
            }
        }
        _ => f["evidence"]["producer"] = json!(["automation:ci", "agent:executor"][pick % 2]),
    }
    f
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]
    #[test]
    fn both_implementations_agree_on_mutated_fixtures(index in 0usize..152, which in any::<u8>(), pick in 0usize..16, twice in any::<bool>(), which2 in any::<u8>()) {
        let base = corpus()[index].clone();
        let mut f = mutate(base, which, pick);
        if twice {
            f = mutate(f, which2, pick / 2);
        }
        // The reference is pinned by the registration and covers the
        // contract's domain; outside it the verifier refuses (the property
        // below), so only well-formed mutations are compared.
        prop_assume!(rha_verifier::malformed(&f).is_none());
        prop_assert_eq!(rha_verifier::evaluate(&f), reference::evaluate(&f), "fixture {} mutation {} {}", index, which % 12, which2 % 12);
    }

    /// Fail closed: deleting or retyping any one member the contract requires
    /// is refused, with `merge_allowed = false` and the member named (Opus 5.5
    /// review of pull request 17, finding 1).
    #[test]
    fn a_fixture_missing_or_mistyping_a_member_is_refused(index in 0usize..152, member in 0usize..MEMBERS.len(), retype in any::<bool>()) {
        let mut f = corpus()[index].clone();
        let (parent, key) = MEMBERS[member];
        let target = match parent {
            "" => &mut f,
            p => &mut f[p],
        };
        prop_assume!(target.is_object() && target.get(key).is_some());
        if retype {
            target[key] = json!(-1.5);
        } else {
            target.as_object_mut().unwrap().remove(key);
        }
        let d = rha_verifier::evaluate(&f);
        prop_assert!(d["malformed"].is_string(), "{parent}.{key}: {d}");
        prop_assert_eq!(&d["merge_allowed"], &json!(false));
        prop_assert!(d["valid_exception"] != json!(true));
    }
}

/// Every member of contract §1's fixture shape, by parent.
const MEMBERS: &[(&str, &str)] = &[
    ("policy", "digest"),
    ("policy", "profile"),
    ("policy", "checks"),
    ("policy", "rules"),
    ("policy", "default_obligations"),
    ("policy", "non_waivable"),
    ("policy", "trusted_producers"),
    ("policy", "required_inputs"),
    ("policy", "exception_authority"),
    ("policy", "acceptance_authority"),
    ("policy", "cooling_off_hours"),
    ("", "local_policies"),
    ("", "base"),
    ("", "candidate_tree"),
    ("", "surface"),
    ("evidence", "producer"),
    ("evidence", "integrity"),
    ("evidence", "subject"),
    ("evidence", "policy"),
    ("evidence", "base"),
    ("evidence", "inputs"),
    ("evidence", "entries"),
    ("", "acceptor"),
    ("", "accountable_change_authority"),
    ("", "now"),
    ("exception", "issuer"),
    ("exception", "authentic"),
    ("exception", "subject"),
    ("exception", "base"),
    ("exception", "policy"),
    ("exception", "waived"),
    ("exception", "issued_at"),
    ("exception", "expires_at"),
    ("exception", "revoked"),
    ("exception", "reason"),
    ("exception", "compensating_control"),
    ("exception", "follow_up"),
];

#[test]
fn the_review_probes_are_refused() {
    let base = || {
        corpus()
            .into_iter()
            .find(|f| f["exception"].is_object())
            .unwrap()
    };
    let mut cases = Vec::new();
    let mut f = base();
    f["exception"].as_object_mut().unwrap().remove("revoked");
    cases.push(("revoked absent", f));
    let mut f = base();
    f.as_object_mut()
        .unwrap()
        .remove("accountable_change_authority");
    cases.push(("accountable_change_authority absent", f));
    let mut f = base();
    f["surface"] = json!([1]);
    cases.push(("surface not strings", f));
    let mut f = base();
    f["surface"] = json!([]);
    cases.push(("surface empty", f));
    let mut f = base();
    f["policy"]["cooling_off_hours"] = json!(-48);
    cases.push(("negative cooling-off", f));
    for (name, f) in cases {
        let d = rha_verifier::evaluate(&f);
        assert!(
            d["malformed"].is_string() && d["merge_allowed"] == json!(false),
            "{name}: {d}"
        );
    }
    // A fractional count is well formed, as V031's negative one is, and is
    // not a pass: a test count is whole. The pinned reference passes it; that
    // disagreement is recorded in the change record.
    let mut f = corpus().into_iter().find(|f| f["id"] == "V001").unwrap();
    f["evidence"]["entries"][0]["selected_tests"] = json!(0.5);
    let d = rha_verifier::evaluate(&f);
    assert!(d["malformed"].is_null(), "{d}");
    assert_eq!(
        (&d["passed"], &d["merge_allowed"]),
        (&json!(false), &json!(false))
    );
}

/// Independent: the root checks the policy triggers on the surface, by the
/// contract's rules, computed without the crate (Lemma 1's precondition).
fn triggered_root_ids(f: &Value) -> Vec<String> {
    let rules = f["policy"]["rules"].as_array().cloned().unwrap_or_default();
    let mut ids = Vec::new();
    let mut unmatched = false;
    for path in f["surface"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let mut hit = false;
        for r in &rules {
            if r["scope"]
                .as_str()
                .is_some_and(|g| rha_verifier::glob(g, path))
            {
                hit = true;
                ids.extend(
                    r["requires"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
            }
        }
        unmatched |= !hit;
    }
    if unmatched {
        ids.extend(
            f["policy"]["default_obligations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }
    ids.sort();
    ids.dedup();
    ids
}

/// Exact number order for the property checks (never through `f64` for two
/// integers).
fn number_cmp(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a.as_u64(), b.as_u64()) {
        (Some(x), Some(y)) => Some(x.cmp(&y)),
        _ => a.as_f64()?.partial_cmp(&b.as_f64()?),
    }
}

fn at_least(key: &str, a: &Value, b: &Value) -> bool {
    match key {
        "delta" => number_cmp(a, b).is_some_and(std::cmp::Ordering::is_le),
        "n" | "executions" | "proptest_cases" => {
            number_cmp(a, b).is_some_and(std::cmp::Ordering::is_ge)
        }
        "selection" => b.as_array().is_some_and(|bs| {
            bs.iter()
                .all(|x| a.as_array().is_some_and(|aa| aa.contains(x)))
        }),
        _ => a == b,
    }
}

/// Independent: whether the evidence passes one required check (outcome and
/// kind validity), recomputed without the crate (Proposition 2).
fn passes(f: &Value, check: &Value) -> bool {
    let entries: Vec<&Value> = f["evidence"]["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["id"] == check["id"])
        .collect();
    let Some(e) = entries.first() else {
        return false;
    };
    e["outcome"] == "passed"
        && match check["kind"].as_str().unwrap_or_default() {
            "test" => e["selected_tests"].as_u64().is_some_and(|n| n > 0),
            "comparison" => e["performed"] == true,
            "corpus" => {
                !check["params"]["corpus_digest"].is_null()
                    && e["corpus_digest"] == check["params"]["corpus_digest"]
            }
            _ => true,
        }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// Lemma 1: without a conflict, every triggered root check is in R_eff,
    /// with parameters at least as strict. Presence is asserted, so an
    /// evaluator that drops an obligation fails here (review round 1, 3).
    #[test]
    fn lemma_1_obligations_only_grow(index in 0usize..152, which in any::<u8>(), pick in 0usize..16) {
        let f = mutate(corpus()[index].clone(), which, pick);
        let d = rha_verifier::evaluate(&f);
        if d["conflict"] == false {
            let r_eff = d["r_eff"].as_array().unwrap();
            for id in triggered_root_ids(&f) {
                let eff = r_eff.iter().find(|c| c["id"] == id.as_str());
                prop_assert!(eff.is_some(), "triggered root check {} is missing from R_eff", id);
                if let Some(root) = f["policy"]["checks"].as_array().into_iter().flatten().find(|c| c["id"] == id.as_str()) {
                    for (k, v) in root["params"].as_object().into_iter().flatten() {
                        prop_assert!(at_least(k, &eff.unwrap()["params"][k.as_str()], v), "{}: {} weaker than root", id, k);
                    }
                }
            }
        }
    }

    /// Lemma 3: R_eff depends on the candidate only through the surface; the
    /// evidence, exception, acceptor and time cannot change it.
    #[test]
    fn lemma_3_evidence_cannot_choose_the_obligations(index in 0usize..152, which in 1u8..11, pick in 0usize..16) {
        let base = corpus()[index].clone();
        let which = if which == 7 { 1 } else { which };
        let mutated = mutate(base.clone(), which, pick);
        let (a, b) = (rha_verifier::evaluate(&base), rha_verifier::evaluate(&mutated));
        prop_assert_eq!(&a["r_eff"], &b["r_eff"]);
        prop_assert_eq!(&a["conflict"], &b["conflict"]);
    }

    /// Proposition 2: no silent pass. When merge is allowed, every required
    /// check either passes, recomputed here independently, or is waived by
    /// the exception and is not non-waivable (review round 1, finding 4).
    #[test]
    fn proposition_2_no_silent_pass(index in 0usize..152, which in any::<u8>(), pick in 0usize..16, twice in any::<bool>()) {
        let mut f = mutate(corpus()[index].clone(), which, pick);
        if twice {
            f = mutate(f, which.wrapping_add(3), pick / 2);
        }
        let d = rha_verifier::evaluate(&f);
        if d["merge_allowed"] == true {
            prop_assert_eq!(&d["conflict"], &json!(false));
            prop_assert!(d["authentic"] == true && d["applicable"] == true && d["complete"] == true);
            prop_assert!(d["eligible"] == true || d["valid_exception"] == true);
            let waived: Vec<&str> = f["exception"]["waived"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            let non_waivable: Vec<&str> = f["policy"]["non_waivable"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
            for check in d["r_eff"].as_array().unwrap() {
                let id = check["id"].as_str().unwrap_or_default();
                prop_assert!(passes(&f, check) || (waived.contains(&id) && !non_waivable.contains(&id)), "{} neither passed nor validly waived", id);
            }
        }
    }
}

#[test]
fn mixed_numbers_at_the_limit_are_incomparable_not_equal() {
    let base = corpus().into_iter().find(|f| f["id"] == "V002").unwrap();
    let mut f = base.clone();
    if let Some(c) = f["policy"]["checks"]
        .as_array_mut()
        .and_then(|a| a.first_mut())
    {
        c["params"]["n"] = json!(9_007_199_254_740_993_u64);
    }
    if let Some(e) = f["evidence"]["entries"]
        .as_array_mut()
        .and_then(|a| a.first_mut())
    {
        e["params"]["n"] = json!(9_007_199_254_740_992.0_f64);
    }
    let d = rha_verifier::evaluate(&f);
    assert_ne!(d["merge_allowed"], json!(true), "{d}");
}
