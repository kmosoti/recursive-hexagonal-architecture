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
