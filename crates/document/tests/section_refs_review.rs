//! Regression tests from the independent review of c556d01 (CHG-010), written before the repair.
//! They pin contract clauses the registered corpus does not exercise.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::{Node, parse};
use library::{RelPath, Source};

fn doc(text: &str) -> document::Document {
    parse(&Source::new(
        RelPath::new("p.md").expect("path"),
        text.to_owned(),
    ))
}

fn links(nodes: &[Node], out: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            Node::Link { href, children } => {
                let mut text = String::new();
                for child in children {
                    if let Node::Text(t) = child {
                        text.push_str(t);
                    }
                }
                out.push((href.clone(), text));
                links(children, out);
            }
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => links(children, out),
            Node::List { items, .. } => items.iter().for_each(|item| links(item, out)),
            _ => {}
        }
    }
}

/// Pages without `§` must parse exactly as before the feature (contract §5, "nothing else
/// changes"). The expected trees were captured with the parser at 4be1a7d, the commit before
/// CHG-010's implementation: the review's inputs plus every page under docs/ without a `§`.
#[test]
fn pages_without_a_section_sign_parse_exactly_as_before() {
    let golden: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("data/pre_section_ref_trees.json"))
            .expect("golden trees");
    assert_eq!(golden.len(), 20);
    for (name, case) in golden {
        let markdown = case["markdown"].as_str().expect("markdown");
        let document = doc(markdown);
        assert_eq!(
            format!("{:?}", document.body),
            case["body"].as_str().expect("body"),
            "{name}: body tree"
        );
        assert_eq!(
            format!("{:?}", document.headings),
            case["headings"].as_str().expect("headings"),
            "{name}: headings"
        );
        assert!(document.section_refs.is_empty(), "{name}");
    }
}

/// A user link can have any destination; one that looks like an internal marker is still an
/// ordinary link, and `parse` stays total (its documented contract).
#[test]
fn a_link_destination_cannot_collide_with_section_reference_internals() {
    for index in 0..3 {
        let href = format!("__rha_internal_section_ref_{index}__");
        let document = doc(&format!("# Page\n\nSee [the notes]({href}).\n"));
        assert!(document.section_refs.is_empty());
        let mut found = Vec::new();
        links(&document.body, &mut found);
        assert_eq!(found, vec![(href.clone(), "the notes".to_owned())]);

        let document = doc(&format!("## 3. Three\n\n[click]({href}) and §3\n"));
        let mut found = Vec::new();
        links(&document.body, &mut found);
        assert!(
            found.contains(&(href.clone(), "click".to_owned())),
            "{found:?}"
        );
        assert!(
            found.contains(&("#3-three".to_owned(), "§3".to_owned())),
            "{found:?}"
        );
        assert_eq!(document.section_refs.len(), 1);
    }
}

/// Range whitespace is U+0020 only (contract §2, as amended before registration).
#[test]
fn a_tab_is_not_range_whitespace() {
    for text in ["§3\t–\t5", "§3\t-5", "§3 –\t5"] {
        let document = doc(&format!("## 3. Three\n## 5. Five\n\nsee {text} here\n"));
        let refs: Vec<_> = document
            .section_refs
            .iter()
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(refs, vec!["§3"], "{text:?}");
    }
}
