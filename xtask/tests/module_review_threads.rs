//! Regressions for the two automated review threads on pull request 18
//! (CHG-007.2): paths in match-arm guards, and `#[cfg(test)]` on associated
//! items other than functions.

use std::collections::BTreeSet;

use xtask::modules::extract::{Extracted, extract};

fn extract_source(label: &str, lib: &str) -> Extracted {
    let root =
        std::env::temp_dir().join(format!("rha-module-threads-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("scratch");
    std::fs::write(root.join("src/lib.rs"), lib).expect("write");
    let extracted = extract(
        &root,
        &root.join("src/lib.rs"),
        "control_crate",
        "2021",
        &BTreeSet::new(),
    )
    .expect("extract");
    let _ = std::fs::remove_dir_all(&root);
    extracted
}

/// The edges from `constraints` to `ordering`, as `test_only` flags.
fn constraint_edges(extracted: &Extracted) -> Vec<bool> {
    extracted
        .edges
        .iter()
        .filter(|e| e.source.ends_with("::constraints") && e.target.contains("::ordering"))
        .map(|e| e.test_only)
        .collect()
}

#[test]
fn a_path_used_only_in_a_match_guard_is_a_production_edge() {
    let extracted = extract_source(
        "guard",
        r"
pub mod ordering {
    pub fn allowed() -> bool { true }
}
pub mod constraints {
    pub fn check(x: u8) -> u8 {
        match x {
            _ if crate::ordering::allowed() => 1,
            _ => 0,
        }
    }
}
",
    );
    let edges = constraint_edges(&extracted);
    assert!(
        !edges.is_empty() && edges.iter().all(|test_only| !test_only),
        "{:?}",
        extracted.edges
    );
}

#[test]
fn cfg_test_on_associated_consts_and_types_marks_their_edges_test_only() {
    let extracted = extract_source(
        "assoc",
        r"
pub mod ordering {
    pub const LIMIT: u8 = 1;
    pub struct Wave;
}
pub mod constraints {
    pub struct Rule;
    pub trait Shape {
        type Unit;
        #[cfg(test)]
        const TRAIT_LIMIT: u8 = crate::ordering::LIMIT;
    }
    impl Rule {
        #[cfg(test)]
        const LIMIT: u8 = crate::ordering::LIMIT;
    }
    impl Shape for Rule {
        #[cfg(test)]
        type Unit = crate::ordering::Wave;
    }
}
",
    );
    let edges = constraint_edges(&extracted);
    assert!(
        !edges.is_empty(),
        "no edge extracted: {:?}",
        extracted.edges
    );
    assert!(
        edges.iter().all(|test_only| *test_only),
        "a test-only associated item produced a production edge: {:?}",
        extracted.edges
    );
}

// --- The Opus 5.5 review of pull request 18 (CHG-007.3) ---
//
// Each probe is the reviewer's input: a legal crate whose rules allow
// `constraints` no dependency. A cross-component path must be a finding, or
// at least a named limitation, never a silent pass.

const RULES: &str = r#"[components]
constraints = "control_crate::constraints"
ordering = "control_crate::ordering"

[allow]
constraints = []
ordering = []

[deny]
cycles = true
child_to_parent_private = true
foreign_internal = true
"#;

/// Extracts and checks a crate; returns the rules findings.
fn check_crate(label: &str, edition: &str, files: &[(&str, &str)]) -> Vec<serde_json::Value> {
    check_crate_with(label, edition, files, RULES)
}

fn check_crate_with(
    label: &str,
    edition: &str,
    files: &[(&str, &str)],
    rules: &str,
) -> Vec<serde_json::Value> {
    let root =
        std::env::temp_dir().join(format!("rha-module-review-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("parent")).expect("dir");
        std::fs::write(file, text).expect("write");
    }
    std::fs::write(root.join("rha-modules.toml"), rules).expect("rules");
    let extracted = extract(
        &root,
        &root.join("src/lib.rs"),
        "control_crate",
        edition,
        &BTreeSet::new(),
    )
    .unwrap_or_else(|e| panic!("{label}: extraction failed: {e}"));
    let checked = xtask::modules::rules::check(
        &root,
        "control_crate",
        &root.join("rha-modules.toml"),
        &extracted,
    )
    .unwrap_or_else(|e| panic!("{label}: check failed: {e}"));
    let _ = std::fs::remove_dir_all(&root);
    checked.findings
}

fn undeclared(findings: &[serde_json::Value]) -> bool {
    findings
        .iter()
        .any(|f| f.to_string().contains("modules.undeclared_dependency"))
}

const ORDERING: &str = "pub mod ordering { pub fn score() -> u8 { 1 } pub const S: u8 = 1; }\n";

#[test]
fn the_control_case_is_a_finding() {
    let lib = format!(
        "pub mod constraints {{ pub fn f() -> u8 {{ crate::ordering::score() }} }}\n{ORDERING}"
    );
    assert!(undeclared(&check_crate(
        "c0",
        "2021",
        &[("src/lib.rs", &lib)]
    )));
}

#[test]
fn a_module_declared_in_a_function_body_or_const_block_is_walked() {
    let in_fn = format!(
        "pub mod constraints {{ pub fn f() -> u8 {{ mod inner {{ pub fn g() -> u8 {{ crate::ordering::score() }} }} inner::g() }} }}\n{ORDERING}"
    );
    assert!(
        undeclared(&check_crate("p1", "2021", &[("src/lib.rs", &in_fn)])),
        "p1"
    );
    let in_const = format!(
        "pub mod constraints {{ pub const X: u8 = {{ mod k {{ pub const Y: u8 = crate::ordering::S; }} k::Y }}; }}\n{ORDERING}"
    );
    assert!(
        undeclared(&check_crate("p7", "2021", &[("src/lib.rs", &in_const)])),
        "p7"
    );
}

#[test]
fn a_path_into_a_known_module_is_an_edge_even_when_the_item_is_not_found() {
    let cases = [
        (
            "p3",
            "pub mod ordering { macro_rules! make { () => { pub fn score() -> u8 { 1 } } } make!(); }",
            "crate::ordering::score()",
        ),
        (
            "p5",
            "pub mod ordering { extern \"C\" { pub fn ext_score() -> u8; } }",
            "unsafe { crate::ordering::ext_score() }",
        ),
        (
            "p6",
            "pub mod ordering { thread_local! { pub static COUNTER: u8 = 1; } }",
            "crate::ordering::COUNTER.with(|c| *c)",
        ),
    ];
    for (label, ordering, call) in cases {
        let lib = format!("pub mod constraints {{ pub fn f() -> u8 {{ {call} }} }}\n{ordering}\n");
        assert!(
            undeclared(&check_crate(label, "2021", &[("src/lib.rs", &lib)])),
            "{label}"
        );
    }
}

#[test]
fn extern_crate_self_aliases_the_crate_root() {
    let lib = format!(
        "extern crate self as me;\npub mod constraints {{ pub fn f() -> u8 {{ me::ordering::score() }} }}\n{ORDERING}"
    );
    assert!(undeclared(&check_crate(
        "p2",
        "2021",
        &[("src/lib.rs", &lib)]
    )));
}

#[test]
fn a_leading_double_colon_is_the_crate_root_in_edition_2015() {
    let lib =
        format!("pub mod constraints {{ pub fn f() -> u8 {{ ::ordering::score() }} }}\n{ORDERING}");
    assert!(undeclared(&check_crate(
        "p29",
        "2015",
        &[("src/lib.rs", &lib)]
    )));
}

#[test]
fn raw_identifiers_are_normalized() {
    let lib = format!(
        "pub mod constraints {{ pub fn f() -> u8 {{ crate::r#ordering::score() }} }}\n{ORDERING}"
    );
    assert!(
        undeclared(&check_crate("p38", "2021", &[("src/lib.rs", &lib)])),
        "p38"
    );
    let lib =
        format!("pub mod constraints {{ pub fn f() -> u8 {{ 1 }} }}\n{ORDERING}mod r#type;\n");
    let findings = check_crate(
        "p39",
        "2021",
        &[("src/lib.rs", &lib), ("src/type.rs", "pub fn t() {}\n")],
    );
    assert!(findings.is_empty(), "p39: {findings:?}");
}

#[test]
fn a_test_function_outside_cfg_test_is_a_test_edge() {
    let lib = format!(
        "pub mod constraints {{ #[test] fn t() {{ assert!(crate::ordering::score() == 1); }} }}\n{ORDERING}"
    );
    let findings = check_crate("p8", "2021", &[("src/lib.rs", &lib)]);
    assert!(!undeclared(&findings), "p8: {findings:?}");
}

// --- The GPT-6 re-review of CHG-007.3 (CHG-007.4): each repair above must not
// raise a false alarm on legal code.

#[test]
fn super_inside_a_function_of_a_block_module_is_the_enclosing_module() {
    let lib = format!(
        "pub mod constraints {{ fn helper() -> u8 {{ 1 }} pub fn f() -> u8 {{ mod inner {{ pub fn g() -> u8 {{ super::helper() }} }} inner::g() }} }}\n{ORDERING}"
    );
    let findings = check_crate("r1", "2021", &[("src/lib.rs", &lib)]);
    assert!(findings.is_empty(), "r1: {findings:?}");
}

#[test]
fn an_absolute_external_path_in_edition_2015_is_external() {
    let lib = format!(
        "pub mod constraints {{ pub fn f() -> usize {{ ::std::mem::size_of::<u8>() }} }}\n{ORDERING}"
    );
    let findings = check_crate("r2", "2015", &[("src/lib.rs", &lib)]);
    assert!(findings.is_empty(), "r2: {findings:?}");
}

#[test]
fn a_local_module_shadows_a_crate_alias() {
    let lib = format!(
        "extern crate self as me;\npub mod constraints {{ mod me {{ pub fn score() -> u8 {{ 1 }} }} pub fn f() -> u8 {{ me::score() }} }}\n{ORDERING}"
    );
    let findings = check_crate("r3", "2021", &[("src/lib.rs", &lib)]);
    assert!(findings.is_empty(), "r3: {findings:?}");
}

#[test]
fn a_raw_import_alias_still_reports_a_foreign_internal_reference() {
    let allowing = RULES.replace("constraints = []", "constraints = [\"ordering\"]");
    let lib = "pub mod constraints { use crate::ordering as r#o; pub fn f() -> u8 { r#o::internal::score() } }\npub mod ordering { pub mod internal { pub fn score() -> u8 { 1 } } }\n";
    let findings = check_crate_with("r4", "2021", &[("src/lib.rs", lib)], &allowing);
    assert!(
        findings
            .iter()
            .any(|f| f.to_string().contains("modules.foreign_internal")),
        "r4: {findings:?}"
    );
}

#[test]
fn a_raw_identifier_inside_a_macro_is_normalized() {
    let lib = format!(
        "pub mod constraints {{ pub fn f() {{ assert!(crate::r#ordering::score() == 1); }} }}\n{ORDERING}"
    );
    let findings = check_crate("r5", "2021", &[("src/lib.rs", &lib)]);
    assert!(undeclared(&findings), "r5: {findings:?}");
    assert!(
        !findings
            .iter()
            .any(|f| f.to_string().contains("child_to_parent_private")),
        "r5: {findings:?}"
    );
}

#[test]
fn super_super_from_a_block_module_function_reaches_the_sibling() {
    let lib = format!(
        "pub mod constraints {{ pub fn f() -> u8 {{ mod inner {{ pub fn g() -> u8 {{ super::super::ordering::score() }} }} inner::g() }} }}\n{ORDERING}"
    );
    assert!(
        undeclared(&check_crate("q3", "2021", &[("src/lib.rs", &lib)])),
        "q3"
    );
}
