//! The pre-registered corpus manifest is complete and well formed (CHG-002).
//!
//! These tests are the mechanism behind the pre-registration. Plan §6 names
//! every case; this file pins the manifest to that list, so a case cannot be
//! dropped, renamed, or have its expected outcome changed without a test
//! failing and a reviewer asking why. Spec §17 wants the cases fixed before
//! the data; a promise nothing checks is not fixed.
//!
//! The expected outcomes are transcribed from plan §6 by hand and appear here
//! a second time on purpose. If someone edits the manifest, this file
//! disagrees; if someone edits both, the diff shows it in one place.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use xtask::corpus::{Expected, Generation, Level, MANIFEST_PATH, Manifest};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

fn manifest() -> Manifest {
    let path = root().join(MANIFEST_PATH);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    Manifest::parse(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

/// Plan §6, crate-level table, in order.
const CRATE_IDS: [&str; 35] = [
    "C01", "C02", "C03", "C04", "C05", "C06", "C07", "C08", "C09", "C10", "C11", "C12", "C13",
    "C14", "C15", "C16", "C17", "C18", "C19", "C20", "C21", "L01", "L02", "L03", "L04", "L05",
    "L06", "L07", "L08", "L09", "L10", "L11", "EM-C01", "EM-C02", "R01",
];

/// Plan §6, module-level table, in order.
const MODULE_IDS: [&str; 27] = [
    "M01", "M02", "M03", "M04", "M05", "M06", "M07", "M08", "M09", "M10", "M11", "M12", "M13",
    "M14", "M15", "M16", "M17", "M18", "M19", "M20", "M21", "L-M01", "L-M02", "EM-M01", "EM-M02",
    "EM-M03", "X-M01",
];

/// The rule ids plan §6 names, plus the two the plan writes in other words:
/// R01's detector is rustc, and X-M01's is an external tool.
const PLAN_RULES: [&str; 18] = [
    "dir.core_to_adapter",
    "dir.core_to_app",
    "dir.non_root_to_adapter",
    "dir.tool_depended_on",
    "effect.core_disallowed_dependency",
    "effect.core_disallowed_dev_dependency",
    "effect.core_build_script",
    "transitive.core_disallowed_dependency",
    "graph.cycle",
    "forbidden.edge",
    "class.unclassified",
    "class.prefix_role_conflict",
    "adapter.missing_port_owner",
    "adapter.port_owner_wrong_kind",
    "meta.unknown_port_owner",
    "modules.cycle",
    "modules.undeclared_dependency",
    "modules.child_to_parent_private",
];

/// `modules.foreign_internal` and `rustc E0603` complete the set; they are
/// listed apart because the first is a W7 rule and the second is a compiler
/// error code, not an xtask rule id.
const OTHER_RULES: [&str; 2] = ["modules.foreign_internal", "rustc E0603"];

#[test]
fn every_case_of_plan_section_6_is_registered_exactly_once() {
    let manifest = manifest();
    let ids = manifest.ids();
    let expected: Vec<&str> = CRATE_IDS.into_iter().chain(MODULE_IDS).collect();

    let registered: BTreeSet<&str> = ids.iter().copied().collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    let missing: Vec<&str> = wanted.difference(&registered).copied().collect();
    let extra: Vec<&str> = registered.difference(&wanted).copied().collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "manifest does not match plan §6: missing {missing:?}, unexpected {extra:?}"
    );
    assert_eq!(
        ids.len(),
        expected.len(),
        "an id is registered more than once: {ids:?}"
    );
}

#[test]
fn the_expected_outcome_counts_are_the_ones_plan_section_6_states() {
    let manifest = manifest();

    let crate_counts = manifest.counts(Level::Crate);
    assert_eq!(crate_counts.get(&Expected::Detect), Some(&22));
    assert_eq!(crate_counts.get(&Expected::NoAlarm), Some(&11));
    assert_eq!(crate_counts.get(&Expected::ExpectedMiss), Some(&2));
    assert_eq!(crate_counts.values().sum::<usize>(), 35);

    // 22, not 21: C01 to C21 are the twenty-one xtask violations, and R01 is a
    // twenty-second crate-level detection whose detector is rustc, not xtask.
    // The harness runs `cargo check --offline` for it and scores it apart.
    let rustc_cases: Vec<&str> = manifest
        .cases
        .iter()
        .filter(|c| c.level == Level::Crate && c.detector.is_some())
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(rustc_cases, vec!["R01"]);
    assert_eq!(
        manifest
            .cases
            .iter()
            .filter(|c| c.level == Level::Crate
                && c.expected == Expected::Detect
                && c.detector.is_none())
            .count(),
        21,
        "plan §6 names twenty-one crate-level violations for the checker"
    );

    let module_counts = manifest.counts(Level::Module);
    assert_eq!(module_counts.get(&Expected::Detect), Some(&17));
    assert_eq!(module_counts.get(&Expected::NoAlarm), Some(&6));
    assert_eq!(module_counts.get(&Expected::ExpectedMiss), Some(&3));
    assert_eq!(module_counts.get(&Expected::Reference), Some(&1));
    assert_eq!(module_counts.values().sum::<usize>(), 27);
}

#[test]
fn every_rule_a_case_names_is_one_the_plan_lists() {
    let manifest = manifest();
    let known: BTreeSet<&str> = PLAN_RULES.into_iter().chain(OTHER_RULES).collect();
    let used = manifest.rules();
    let unknown: Vec<&str> = used.difference(&known).copied().collect();
    assert!(
        unknown.is_empty(),
        "cases name rules the plan does not list: {unknown:?}"
    );
}

#[test]
fn the_manifest_has_no_defects() {
    let manifest = manifest();
    let defects = manifest.defects();
    assert!(
        defects.is_empty(),
        "{}",
        defects
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_crate_level_case_declares_a_workspace_chg_004_can_generate() {
    let manifest = manifest();
    for case in manifest.cases.iter().filter(|c| c.level == Level::Crate) {
        assert_eq!(
            case.generation,
            Generation::Declared,
            "{}: a crate-level case is generated from this file, not authored",
            case.id
        );
        assert!(
            !case.crates.is_empty(),
            "{}: declares no crates, so nothing can be generated",
            case.id
        );
        for krate in &case.crates {
            assert!(!krate.name.is_empty(), "{}: a crate has no name", case.id);
        }
    }
}

/// The four shapes that hide one edge behind a different Cargo feature. A
/// checker that parses Cargo.toml text instead of reading `cargo metadata`
/// misses one of them, so the manifest must actually declare all four.
#[test]
fn the_dependency_shapes_of_c15_to_c18_are_each_declared() {
    let manifest = manifest();
    let dep_of = |id: &str| {
        manifest
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id} is registered"))
            .crates
            .iter()
            .find(|k| k.name == "core-a")
            .unwrap_or_else(|| panic!("{id} seeds core-a"))
            .deps
            .first()
            .unwrap_or_else(|| panic!("{id}'s core-a declares a dependency"))
            .clone()
    };
    assert_eq!(dep_of("C15").target.as_deref(), Some("cfg(unix)"));
    assert!(dep_of("C16").optional);
    assert_eq!(dep_of("C17").rename.as_deref(), Some("ax"));
    assert!(dep_of("C18").inherit);
    for id in ["C15", "C16", "C17", "C18"] {
        assert_eq!(dep_of(id).name, "adapter-x", "{id} seeds the same edge");
    }
}

/// C10 needs a member with no metadata at all; C11 needs one whose metadata
/// contradicts its prefix. A generator that always writes a role makes both
/// cases untestable, so the manifest must distinguish absent from present.
#[test]
fn the_classification_cases_declare_absent_and_conflicting_metadata() {
    let manifest = manifest();
    let krate = |case_id: &str, name: &str| {
        manifest
            .cases
            .iter()
            .find(|c| c.id == case_id)
            .and_then(|c| c.crates.iter().find(|k| k.name == name))
            .unwrap_or_else(|| panic!("{case_id} seeds {name}"))
            .clone()
    };
    assert_eq!(krate("C10", "mystery").role, None);
    assert_eq!(krate("C11", "adapter-x").role.as_deref(), Some("core"));
    assert_eq!(
        krate("L11", "planner-adapter-utils").role.as_deref(),
        Some("core")
    );
}

/// The pre-registration says what happens when a run disagrees with it. Both
/// directions are stated, and neither ends in editing the manifest.
#[test]
fn the_grading_rules_state_what_a_disagreement_does() {
    let manifest = manifest();
    let grading = &manifest.grading;
    assert!(
        grading.miss_is_terminal.contains("never edits this file"),
        "a miss must not be resolvable by editing the corpus"
    );
    assert!(
        grading
            .expected_miss_surprise
            .contains("not silently re-registered"),
        "a surprise detection must be escalated, not absorbed"
    );
    assert_eq!(grading.open_at, "DP-1.1");
}

/// No checker code exists yet, and no fixture workspace is committed. This is
/// what makes the manifest a pre-registration rather than a description.
#[test]
fn no_fixture_workspace_and_no_checker_is_committed_yet() {
    let root = root();
    for dir in ["xtask/tests/corpus/crate", "xtask/tests/corpus/module"] {
        assert!(
            !root.join(dir).exists(),
            "{dir} exists; fixtures are generated in CHG-004 and authored in CHG-007, not committed here"
        );
    }
    assert!(
        !root.join("xtask/src/graph").exists(),
        "xtask/src/graph exists; the crate-graph checker is CHG-003, after this pre-registration"
    );
    let stub = std::fs::read_to_string(root.join("xtask/src/architecture.rs"))
        .expect("the architecture stub is readable");
    assert!(
        stub.contains("EXIT_NOT_RUN"),
        "xtask/src/architecture.rs no longer reports not_run; CHG-002 must not implement the checker"
    );
}
