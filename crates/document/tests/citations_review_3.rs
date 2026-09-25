//! Regression tests from the independent review of 14a1d57 (CHG-011 repair 2), written before the
//! third repair. In a tight list item, a label is "followed by the end of the paragraph" (contract
//! section 1) whenever any block follows it, not only a nested list or block quote. Expected values
//! are the registered oracle's (`xtask/tests/corpus/citations/reference.py`, `analyze_page`).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::{Citation, ReferenceEntry, parse};
use library::{RelPath, Source};

fn doc(text: &str) -> document::Document {
    parse(&Source::new(
        RelPath::new("p.md").expect("path"),
        text.to_owned(),
    ))
}

fn assert_entry_then_citation(markdown: &str, citation_line: usize) {
    let d = doc(markdown);
    assert_eq!(
        d.reference_entries,
        vec![ReferenceEntry {
            label: "R1".to_owned(),
            anchor: "ref-r1".to_owned(),
            line: 1,
        }],
        "{markdown:?}"
    );
    assert_eq!(
        d.citations,
        vec![Citation {
            label: "R1".to_owned(),
            target: Some("ref-r1".to_owned()),
            line: citation_line,
        }],
        "{markdown:?}"
    );
}

#[test]
fn bare_label_before_a_fenced_code_block_in_a_tight_item_is_an_entry() {
    assert_entry_then_citation("- [R1]\n  ```\n  code\n  ```\n\nSee [R1].\n", 6);
}

#[test]
fn bare_label_before_a_table_in_a_tight_item_is_an_entry() {
    assert_entry_then_citation(
        "- [R1]\n  | a | b |\n  |---|---|\n  | c | d |\n\nSee [R1].\n",
        6,
    );
}

#[test]
fn bare_label_before_a_heading_in_a_tight_item_is_an_entry() {
    assert_entry_then_citation("- [R1]\n  # H\n\nSee [R1].\n", 4);
}

#[test]
fn bare_label_before_a_rule_in_a_tight_item_is_an_entry() {
    assert_entry_then_citation("- [R1]\n  ***\n\nSee [R1].\n", 4);
}

#[test]
fn bare_label_before_an_html_block_in_a_tight_item_is_an_entry() {
    assert_entry_then_citation("- [R1]\n  <div>x</div>\n\nSee [R1].\n", 4);
}

#[test]
fn inline_html_directly_after_a_label_does_not_end_the_paragraph() {
    let d = doc("- [R1]<b>x</b>\n\nSee [R1].\n");
    assert!(d.reference_entries.is_empty(), "{:?}", d.reference_entries);
    assert_eq!(d.citations.len(), 2);
    assert!(d.citations.iter().all(|c| c.target.is_none()));
}
