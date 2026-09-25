//! Regression tests from the independent review of 352b0f1 (CHG-011), written before the repair.
//! They pin contract clauses the registered corpus does not exercise: first-wins and document
//! order for entries in tight list items with nested blocks (contract sections 1 and 3), and
//! citations directly after `!` that are not images (section 2).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::{Citation, Diagnostic, Node, ReferenceEntry, parse};
use library::{RelPath, Source};

fn doc(text: &str) -> document::Document {
    parse(&Source::new(
        RelPath::new("p.md").expect("path"),
        text.to_owned(),
    ))
}

fn entry(label: &str, line: usize) -> ReferenceEntry {
    ReferenceEntry {
        label: label.to_owned(),
        anchor: format!("ref-{}", label.to_lowercase()),
        line,
    }
}

fn cited(label: &str, line: usize) -> Citation {
    Citation {
        label: label.to_owned(),
        target: Some(format!("ref-{}", label.to_lowercase())),
        line,
    }
}

fn duplicates(d: &document::Document) -> Vec<(String, usize, usize)> {
    d.diagnostics
        .iter()
        .filter_map(|x| match x {
            Diagnostic::DuplicateReferenceEntry {
                label,
                first_line,
                second_line,
            } => Some((label.clone(), *first_line, *second_line)),
            _ => None,
        })
        .collect()
}

fn text_of(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Link { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children) => out.push_str(&text_of(children)),
            _ => {}
        }
    }
    out
}

/// For every `Node::Anchor`, its id and the text of the nodes that follow it in the vector that
/// holds it: which paragraph the anchor was attached to.
fn anchors(nodes: &[Node], out: &mut Vec<(String, String)>) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            Node::Anchor { id } => out.push((id.clone(), text_of(&nodes[i + 1..]))),
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::BlockQuote { children, .. } => anchors(children, out),
            Node::List { items, .. } => {
                for item in items {
                    anchors(item, out);
                }
            }
            _ => {}
        }
    }
}

fn links(nodes: &[Node], out: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            Node::Link { href, children } => out.push((href.clone(), text_of(children))),
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::BlockQuote { children, .. } => links(children, out),
            Node::List { items, .. } => {
                for item in items {
                    links(item, out);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn outer_tight_list_entry_wins_over_nested_list_entry() {
    let d = doc("- [R1] outer\n  - [R1] inner\n\nsee [R1]\n");
    assert_eq!(d.reference_entries, vec![entry("R1", 1)]);
    assert_eq!(duplicates(&d), vec![("R1".to_owned(), 1, 2)]);
    assert_eq!(d.citations, vec![cited("R1", 2), cited("R1", 4)]);
    let mut found = Vec::new();
    anchors(&d.body, &mut found);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "ref-r1");
    assert!(
        found[0].1.starts_with("[R1] outer"),
        "the anchor belongs to the outer entry: {found:?}"
    );
}

#[test]
fn outer_tight_list_entry_wins_over_nested_block_quote_entry() {
    let d = doc("- [R3] outer\n  > [R3] quoted in item\n");
    assert_eq!(d.reference_entries, vec![entry("R3", 1)]);
    assert_eq!(duplicates(&d), vec![("R3".to_owned(), 1, 2)]);
    assert_eq!(d.citations, vec![cited("R3", 2)]);
}

#[test]
fn entries_in_nested_tight_lists_are_in_document_order() {
    let d = doc("- [R1] outer\n  - [R2] inner\n\nsee [R1] [R2]\n");
    assert_eq!(d.reference_entries, vec![entry("R1", 1), entry("R2", 2)]);
    assert!(duplicates(&d).is_empty());
    assert_eq!(d.citations, vec![cited("R1", 4), cited("R2", 4)]);
}

#[test]
fn citation_directly_after_an_exclamation_mark_is_recognized() {
    let d = doc("wow![R1] and x ![R1]\n\n[R1] a\n");
    assert_eq!(d.reference_entries, vec![entry("R1", 3)]);
    assert_eq!(d.citations, vec![cited("R1", 1), cited("R1", 1)]);
    let mut found = Vec::new();
    links(&d.body, &mut found);
    assert_eq!(
        found,
        vec![
            ("#ref-r1".to_owned(), "[R1]".to_owned()),
            ("#ref-r1".to_owned(), "[R1]".to_owned())
        ]
    );
    let Node::Paragraph(first) = &d.body[0] else {
        panic!("first block is a paragraph: {:?}", d.body[0]);
    };
    assert_eq!(
        text_of(first),
        "wow![R1] and x ![R1]",
        "no text is lost or added"
    );
}

#[test]
fn bang_citation_at_paragraph_start_is_a_citation_not_an_entry() {
    let d = doc("![R1]\n\n[R1] a\n");
    assert_eq!(d.reference_entries, vec![entry("R1", 3)]);
    assert_eq!(d.citations, vec![cited("R1", 1)]);
}

#[test]
fn a_real_image_is_still_excluded() {
    let d = doc("![R1](x.png)\n\n[R1] a\n");
    assert_eq!(d.reference_entries, vec![entry("R1", 3)]);
    assert!(d.citations.is_empty(), "{:?}", d.citations);
    assert!(matches!(d.body[0], Node::Paragraph(ref c) if matches!(c[0], Node::Image { .. })));
}
