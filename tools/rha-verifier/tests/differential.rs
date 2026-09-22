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
    match which % 10 {
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
        prop_assert_eq!(rha_verifier::evaluate(&f), reference::evaluate(&f), "fixture {} mutation {} {}", index, which % 10, which2 % 10);
    }
}

/// Whether `a` is at least as strict as `b` for the contract's keys.
fn at_least(key: &str, a: &Value, b: &Value) -> bool {
    match key {
        "delta" => a.as_f64() <= b.as_f64(),
        "n" | "executions" | "proptest_cases" => a.as_f64() >= b.as_f64(),
        "selection" => b.as_array().is_some_and(|bs| {
            bs.iter()
                .all(|x| a.as_array().is_some_and(|aa| aa.contains(x)))
        }),
        _ => a == b,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// Lemma 1: without a conflict, every triggered root check appears in
    /// R_eff with parameters at least as strict (obligations only grow).
    #[test]
    fn lemma_1_obligations_only_grow(index in 0usize..152, which in any::<u8>(), pick in 0usize..16) {
        let f = mutate(corpus()[index].clone(), which, pick);
        let d = rha_verifier::evaluate(&f);
        if d["conflict"] == false {
            let r_eff = d["r_eff"].as_array().unwrap();
            for root in f["policy"]["checks"].as_array().into_iter().flatten() {
                if let Some(eff) = r_eff.iter().find(|c| c["id"] == root["id"]) {
                    for (k, v) in root["params"].as_object().into_iter().flatten() {
                        prop_assert!(at_least(k, &eff["params"][k.as_str()], v), "{}: {} weaker than root", root["id"], k);
                    }
                }
            }
        }
    }

    /// Lemma 3: R_eff depends on the candidate only through the surface; the
    /// evidence, exception, acceptor and time cannot change it.
    #[test]
    fn lemma_3_evidence_cannot_choose_the_obligations(index in 0usize..152, which in 1u8..10, pick in 0usize..16) {
        let base = corpus()[index].clone();
        let mutated = mutate(base.clone(), if which == 7 { 1 } else { which }, pick);
        let (a, b) = (rha_verifier::evaluate(&base), rha_verifier::evaluate(&mutated));
        prop_assert_eq!(&a["r_eff"], &b["r_eff"]);
        prop_assert_eq!(&a["conflict"], &b["conflict"]);
    }

    /// Proposition 2: no silent pass. If merge is allowed, then either every
    /// required check passed on authentic, applicable, complete evidence, or a
    /// valid exception covers every non-passing check and none is non-waivable.
    #[test]
    fn proposition_2_no_silent_pass(index in 0usize..152, which in any::<u8>(), pick in 0usize..16, twice in any::<bool>()) {
        let mut f = mutate(corpus()[index].clone(), which, pick);
        if twice {
            f = mutate(f, which.wrapping_add(3), pick / 2);
        }
        let d = rha_verifier::evaluate(&f);
        if d["merge_allowed"] == true {
            prop_assert!(d["authentic"] == true && d["applicable"] == true && d["complete"] == true);
            prop_assert!(d["eligible"] == true || d["valid_exception"] == true);
            if d["eligible"] != true {
                let waived: Vec<&Value> = f["exception"]["waived"].as_array().unwrap().iter().collect();
                let nw: Vec<&Value> = f["policy"]["non_waivable"].as_array().into_iter().flatten().collect();
                prop_assert!(waived.iter().all(|w| !nw.contains(w)));
            }
        }
    }
}
