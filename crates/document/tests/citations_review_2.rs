//! Regression tests from the independent review of e8dda29 (CHG-011 repair), written before the
//! second repair. A `![` with no citation must keep the tree it had before the feature (contract
//! section 5), and a bare label ending a tight list item's text is an entry (section 1: a label
//! "followed by a space or the end of the paragraph").
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::{Citation, Node, ReferenceEntry, parse};
use library::{RelPath, Source};

fn doc(text: &str) -> document::Document {
    parse(&Source::new(
        RelPath::new("p.md").expect("path"),
        text.to_owned(),
    ))
}

fn text(t: &str) -> Node {
    Node::Text(t.to_owned())
}

fn entry(label: &str, line: usize) -> ReferenceEntry {
    ReferenceEntry {
        label: label.to_owned(),
        anchor: format!("ref-{}", label.to_lowercase()),
        line,
    }
}

#[test]
fn bang_bracket_without_a_citation_keeps_the_pre_feature_tree() {
    // The tree the parser produced before citations existed (checked at b554bba, whose parse
    // matches 352b0f1 and e2a2f95 on this page).
    let d = doc("wow![x] and ![`c`] here\n");
    assert_eq!(
        d.body,
        vec![Node::Paragraph(vec![
            text("wow"),
            text("!["),
            text("x"),
            text("]"),
            text(" and "),
            text("!["),
            Node::Code("c".to_owned()),
            text("]"),
            text(" here"),
        ])]
    );
    assert!(d.citations.is_empty());
}

#[test]
fn inner_attribute_text_keeps_the_pre_feature_tree() {
    let d = doc("Use #![cfg(test)] here.\n");
    assert_eq!(
        d.body,
        vec![Node::Paragraph(vec![
            text("Use #"),
            text("!["),
            text("cfg(test)"),
            text("]"),
            text(" here."),
        ])]
    );
}

#[test]
fn bare_label_in_a_tight_item_before_a_nested_list_is_an_entry() {
    let d = doc("- [R1]\n  - [R2] x\n\nsee [R1]\n");
    assert_eq!(d.reference_entries, vec![entry("R1", 1), entry("R2", 2)]);
    assert_eq!(
        d.citations,
        vec![Citation {
            label: "R1".to_owned(),
            target: Some("ref-r1".to_owned()),
            line: 4,
        }]
    );
}

#[test]
fn bare_label_in_a_tight_item_before_a_nested_block_quote_is_an_entry() {
    let d = doc("- [R3]\n  > quoted\n");
    assert_eq!(d.reference_entries, vec![entry("R3", 1)]);
}
