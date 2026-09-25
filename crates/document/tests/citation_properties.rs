//! Adversarial challenge test suite for Milestone 1 (crates/document).
//! Stress tests Requirement R2 ("Nothing else changes"), bracket edge cases,
//! and AST invariants (`Node::Anchor` leading positioning across containers).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::{Diagnostic, Document, Node, parse};
use library::{RelPath, Source};

fn parse_doc(md: &str) -> Document {
    let src = Source::new(RelPath::new("test.md").unwrap(), md.to_string());
    parse(&src)
}

fn assert_anchor_leading_invariants(nodes: &[Node], path: &str) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            Node::Anchor { id } => {
                assert_eq!(
                    i, 0,
                    "INVARIANT VIOLATION at {path}: Node::Anchor {id:?} must be at index 0, found at index {i}"
                );
            }
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => {
                assert_anchor_leading_invariants(children, &format!("{path} -> child"));
            }
            Node::List { items, .. } => {
                for (item_idx, item) in items.iter().enumerate() {
                    assert_anchor_leading_invariants(
                        item,
                        &format!("{path} -> list_item[{item_idx}]"),
                    );
                }
            }
            Node::Table { head, rows, .. } => {
                for (h_idx, cell) in head.iter().enumerate() {
                    assert_anchor_leading_invariants(cell, &format!("{path} -> th[{h_idx}]"));
                }
                for (r_idx, row) in rows.iter().enumerate() {
                    for (c_idx, cell) in row.iter().enumerate() {
                        assert_anchor_leading_invariants(
                            cell,
                            &format!("{path} -> row[{r_idx}][{c_idx}]"),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Golden pre-citation corpus verification (44 cases)
// ---------------------------------------------------------------------------
#[test]
fn test_all_44_golden_pre_citation_cases() {
    let golden: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("data/pre_citation_trees.json")).expect("golden trees");
    assert_eq!(golden.len(), 44);

    for (name, case) in golden {
        let markdown = case["markdown"].as_str().expect("markdown");
        let doc = parse_doc(markdown);

        assert_eq!(
            format!("{:?}", doc.body),
            case["body"].as_str().expect("body"),
            "Mismatch in body AST for golden case: {name}"
        );
        assert_eq!(
            format!("{:?}", doc.headings),
            case["headings"].as_str().expect("headings"),
            "Mismatch in headings for golden case: {name}"
        );
        assert!(doc.citations.is_empty(), "{name}: citations must be empty");
        assert!(
            doc.reference_entries.is_empty(),
            "{name}: reference entries must be empty"
        );
        assert_anchor_leading_invariants(&doc.body, &name);
    }
}

// ---------------------------------------------------------------------------
// 2. Tricky bracket combinations without citations (R2: Nothing else changes)
// ---------------------------------------------------------------------------
#[test]
fn test_tricky_brackets_without_citations() {
    let cases = [
        // Unmatched brackets
        "[",
        "]",
        "[[",
        "]]",
        "[[[",
        "]]]",
        "[ hello",
        "world ]",
        "[ hello [ world ]",
        "hello [ world ] again",
        "[ hello [ world ] ]",
        "[a [b [c] d] e]",
        "[[a]]",
        "[[[a]]]",
        "a] b [c",
        "[ [ [ ] ] ]",
        "[ ] [ ]",
        "[]",
        "[ ]",
        "[\n]",
        "[\r\n]",
        "[\t]",
        // Bracketed non-citation patterns
        "[b]",
        "[foo]",
        "[123]",
        "[r1]",
        "[R]",
        "[R1A]",
        "[R1Z]",
        "[R1ab]",
        "[R12ab]",
        "[R-1]",
        "[R+1]",
        "[R 1]",
        "[R1 ]",
        "[ R1 ]",
        "[R\t1]",
        "[R1\t]",
        "[\tR1]",
        "[R1.2]",
        "[R1, R2]",
        "[R1,R2]",
        "[R1; R2]",
        // Escaped brackets
        r"\[foo\]",
        r"\[R1\]",
        r"\[R1\] and [b]",
        // Code spans
        "`[`",
        "`]`",
        "`[b]`",
        "`[R1]`",
        "`[R1][R2]`",
        "code: `[R57]` should not cite",
        // Fenced code
        "```\n[R1]\n[b]\n```",
        "```rust\nlet x = [1, 2, 3];\n```",
        // Indented code
        "    [R1]\n    [b]",
        // Links
        "[link](https://example.com)",
        "[link [nested]](https://example.com)",
        "[link](https://example.com/[foo])",
        "![image](image.png)",
        "![image [nested]](image.png)",
        // Shortcut link
        "[ref]: https://example.com\n\n[ref]",
        "[R1]: https://example.com\n\n[R1]", // Shortcut link suppresses citation
        // Wikilinks
        "[[Page]]",
        "[[Page#section]]",
        "[[Page|alias]]",
        "[[Page|[foo]]]",
        // Transclusions
        "{{page}}",
        "{{page#section}}",
        "{{page#section|Title}}",
        // §-references with brackets
        "§1 [foo]",
        "[foo] §1",
        "[§1]",
        "§1 [b]",
        // Formatting
        "*[b]*",
        "**[b]**",
        "***[b]***",
        "~~[b]~~",
        // Headings
        "# Heading [b]",
        "## Heading [foo [bar]]",
        "### [b]",
    ];

    for case in cases {
        let doc = parse_doc(case);
        assert!(
            doc.citations.is_empty(),
            "Expected 0 citations for case {case:?}, got: {:?}",
            doc.citations
        );
        assert!(
            doc.reference_entries.is_empty(),
            "Expected 0 entries for case {case:?}, got: {:?}",
            doc.reference_entries
        );
        assert_anchor_leading_invariants(&doc.body, case);
    }
}

// ---------------------------------------------------------------------------
// 3. Headings, slugs, and TOC preservation
// ---------------------------------------------------------------------------
#[test]
fn test_headings_and_slugs_invariants() {
    let md = "# Title with `code` and [b]\n\n## Subheading with [foo] & bar\n\n### Heading with [R1]\n\n#### Section §1.2 [test]\n";
    let doc = parse_doc(md);

    assert_eq!(doc.headings.len(), 4);
    assert_eq!(doc.headings[0].text, "Title with code and [b]");
    assert_eq!(doc.headings[0].slug, "title-with-code-and-b");
    assert_eq!(doc.headings[1].text, "Subheading with [foo] & bar");
    assert_eq!(doc.headings[1].slug, "subheading-with-foo--bar");
    // Headings never recognize citations
    assert_eq!(doc.headings[2].text, "Heading with [R1]");
    assert_eq!(doc.headings[2].slug, "heading-with-r1");
    assert!(
        doc.citations.is_empty(),
        "Headings must not yield citations"
    );
    assert!(
        doc.reference_entries.is_empty(),
        "Headings must not yield reference entries"
    );
}

// ---------------------------------------------------------------------------
// 4. Node::Anchor index 0 positioning invariant across all container types
// ---------------------------------------------------------------------------
#[test]
fn test_anchor_index_0_in_all_container_types() {
    // Top-level paragraph
    {
        let doc = parse_doc("[R1] Top-level paragraph entry.\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, "R1");
        assert_eq!(doc.reference_entries[0].anchor, "ref-r1");
        assert_anchor_leading_invariants(&doc.body, "top-level paragraph");
        match &doc.body[0] {
            Node::Paragraph(children) => {
                assert!(matches!(&children[0], Node::Anchor { id } if id == "ref-r1"));
                assert!(matches!(&children[1], Node::Text(s) if s == "[R1]"));
            }
            other => panic!("Expected Paragraph, got {other:?}"),
        }
    }

    // Tight unordered list item
    {
        let doc = parse_doc("- [R1] Tight unordered list entry\n- Plain item\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "tight unordered list");
        match &doc.body[0] {
            Node::List { items, .. } => {
                assert!(matches!(&items[0][0], Node::Anchor { id } if id == "ref-r1"));
                assert!(matches!(&items[0][1], Node::Text(s) if s == "[R1]"));
            }
            other => panic!("Expected List, got {other:?}"),
        }
    }

    // Tight ordered list item
    {
        let doc = parse_doc("1. [R1] Tight ordered list entry\n2. Plain item\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "tight ordered list");
        match &doc.body[0] {
            Node::List { items, start } => {
                assert_eq!(*start, Some(1));
                assert!(matches!(&items[0][0], Node::Anchor { id } if id == "ref-r1"));
                assert!(matches!(&items[0][1], Node::Text(s) if s == "[R1]"));
            }
            other => panic!("Expected List, got {other:?}"),
        }
    }

    // Loose unordered list item (contains Paragraphs)
    {
        let doc = parse_doc("- [R1] Loose unordered list entry\n\n  Second paragraph in item.\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "loose unordered list");
        match &doc.body[0] {
            Node::List { items, .. } => match &items[0][0] {
                Node::Paragraph(children) => {
                    assert!(matches!(&children[0], Node::Anchor { id } if id == "ref-r1"));
                    assert!(matches!(&children[1], Node::Text(s) if s == "[R1]"));
                }
                other => panic!("Expected Paragraph inside loose item, got {other:?}"),
            },
            other => panic!("Expected List, got {other:?}"),
        }
    }

    // Block quote
    {
        let doc = parse_doc("> [R1] Block quote entry\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "block quote");
        match &doc.body[0] {
            Node::BlockQuote { children, .. } => match &children[0] {
                Node::Paragraph(para_children) => {
                    assert!(matches!(&para_children[0], Node::Anchor { id } if id == "ref-r1"));
                    assert!(matches!(&para_children[1], Node::Text(s) if s == "[R1]"));
                }
                other => panic!("Expected Paragraph inside BlockQuote, got {other:?}"),
            },
            other => panic!("Expected BlockQuote, got {other:?}"),
        }
    }

    // Nested block quote
    {
        let doc = parse_doc("> > [R1] Nested block quote entry\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "nested block quote");
    }

    // Block quote containing list containing entry
    {
        let doc = parse_doc("> - [R1] List item inside block quote\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "list in blockquote");
    }

    // List containing block quote containing entry
    {
        let doc = parse_doc("- > [R1] Blockquote inside list item\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_anchor_leading_invariants(&doc.body, "blockquote in list");
    }

    // Table cell: NEVER an entry!
    {
        let doc = parse_doc("| [R1] Cell content |\n|---|\n| [R2] Second cell |\n");
        assert!(
            doc.reference_entries.is_empty(),
            "Table cells must NEVER be entries"
        );
        assert_eq!(doc.citations.len(), 2, "Table cells recognize citations");
        assert_anchor_leading_invariants(&doc.body, "table cells");
    }
}

// ---------------------------------------------------------------------------
// 5. Entry edge cases (soft breaks, hard breaks, EOF, trailing characters)
// ---------------------------------------------------------------------------
#[test]
fn test_entry_edge_cases() {
    // Alone on a line (EOF / paragraph end)
    {
        let doc = parse_doc("[R1]\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, "R1");
        assert_anchor_leading_invariants(&doc.body, "lone entry");
    }

    // Soft break after label
    {
        let doc = parse_doc("[R1]\nContinuing on next line.\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, "R1");
        assert_anchor_leading_invariants(&doc.body, "soft break entry");
    }

    // Hard break after label
    {
        let doc = parse_doc("[R1]  \nContinuing after hard break.\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, "R1");
        assert_anchor_leading_invariants(&doc.body, "hard break entry");
    }

    // Lowercase letter suffix
    {
        let doc = parse_doc("[R12a] Entry with letter suffix.\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, "R12a");
        assert_eq!(doc.reference_entries[0].anchor, "ref-r12a");
    }

    // Glued text after label: NOT an entry!
    {
        let doc = parse_doc("[R1]glued is not an entry.\n");
        assert!(doc.reference_entries.is_empty());
        assert_eq!(doc.citations.len(), 1);
        assert_eq!(doc.citations[0].label, "R1");
        assert_eq!(doc.citations[0].target, None);
    }
}

// ---------------------------------------------------------------------------
// 6. Resolution, link creation, and diagnostics
// ---------------------------------------------------------------------------
#[test]
fn test_resolution_and_diagnostics() {
    // Resolved citation becomes Link
    {
        let doc = parse_doc("[R1] Reference text\n\nCitation here: [R1].\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.citations.len(), 1);
        assert_eq!(doc.citations[0].label, "R1");
        assert_eq!(doc.citations[0].target, Some("ref-r1".to_string()));
        assert!(doc.diagnostics.is_empty());

        match &doc.body[1] {
            Node::Paragraph(children) => {
                let link = &children[1];
                match link {
                    Node::Link {
                        href,
                        children: lchildren,
                    } => {
                        assert_eq!(href, "#ref-r1");
                        assert_eq!(lchildren, &[Node::Text("[R1]".to_string())]);
                    }
                    other => panic!("Expected Node::Link, got {other:?}"),
                }
            }
            other => panic!("Expected Paragraph, got {other:?}"),
        }
    }

    // Unresolved citation with NO entries on page -> NO diagnostic
    {
        let doc = parse_doc("Citation without entry: [R1].\n");
        assert!(doc.reference_entries.is_empty());
        assert_eq!(doc.citations.len(), 1);
        assert_eq!(doc.citations[0].target, None);
        assert!(
            doc.diagnostics.is_empty(),
            "No diagnostic when 0 entries on page"
        );
        match &doc.body[0] {
            Node::Paragraph(children) => {
                assert!(matches!(&children[1], Node::Text(s) if s == "[R1]"));
            }
            other => panic!("Expected Paragraph, got {other:?}"),
        }
    }

    // Unresolved citation WITH an entry on page -> emits UnresolvedCitation
    {
        let doc = parse_doc("[R1] Entry 1\n\nCitation to [R2].\n");
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.citations.len(), 1);
        assert_eq!(doc.citations[0].label, "R2");
        assert_eq!(doc.citations[0].target, None);
        assert_eq!(
            doc.diagnostics,
            vec![Diagnostic::UnresolvedCitation {
                label: "R2".to_string(),
                line: 3,
            }]
        );
    }

    // Duplicate reference entry -> emits DuplicateReferenceEntry, second demoted to citation
    {
        let doc = parse_doc(
            "[R1] First entry (line 1)\n\n[R1] Second entry (line 3)\n\n[R1] Third entry (line 5)\n",
        );
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].line, 1);
        // Both duplicates demoted to citations resolved to first entry
        assert_eq!(doc.citations.len(), 2);
        assert_eq!(doc.citations[0].target, Some("ref-r1".to_string()));
        assert_eq!(doc.citations[1].target, Some("ref-r1".to_string()));

        // Both diagnostics cite first_line: 1
        assert_eq!(
            doc.diagnostics,
            vec![
                Diagnostic::DuplicateReferenceEntry {
                    label: "R1".to_string(),
                    first_line: 1,
                    second_line: 3,
                },
                Diagnostic::DuplicateReferenceEntry {
                    label: "R1".to_string(),
                    first_line: 1,
                    second_line: 5,
                },
            ]
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Stress tests: huge numbers, nested brackets, glued citations, unicode
// ---------------------------------------------------------------------------
#[test]
fn test_stress_and_adversarial_inputs() {
    // Very large citation number
    {
        let big_num = "999999999999999999999999999999999999999999999999999999999999";
        let input = format!("[R{big_num}] Very big number citation\n");
        let doc = parse_doc(&input);
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, format!("R{big_num}"));
        assert_eq!(doc.reference_entries[0].anchor, format!("ref-r{big_num}"));
        assert_anchor_leading_invariants(&doc.body, "big number");
    }

    // Deeply nested brackets
    {
        let input = "[[[[[[[[[[hello]]]]]]]]]]";
        let doc = parse_doc(input);
        assert!(doc.citations.is_empty());
        assert!(doc.reference_entries.is_empty());
        assert_anchor_leading_invariants(&doc.body, "nested brackets");
    }

    // Chained and glued citations with separate paragraphs
    {
        let input = "[R1] First\n\n[R2] Second\n\n[R3] Third\n\nGlued: [R1][R2][R3] and range: [R1]-[R2]-[R3] and word[R1]glued\n";
        let doc = parse_doc(input);
        assert_eq!(doc.reference_entries.len(), 3);
        assert_eq!(doc.citations.len(), 7); // 3 glued + 3 range + 1 word glued
        for cit in &doc.citations {
            assert!(
                cit.target.is_some(),
                "Citation {} should resolve",
                cit.label
            );
        }
        assert_anchor_leading_invariants(&doc.body, "glued and range");
    }

    // Soft-break within single paragraph: only first label is entry, subsequent lines are citations
    {
        let input = "[R1] First line\n[R2] Second line in same paragraph\n";
        let doc = parse_doc(input);
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.reference_entries[0].label, "R1");
        assert_eq!(doc.citations.len(), 1);
        assert_eq!(doc.citations[0].label, "R2");
        assert_eq!(doc.citations[0].target, None);
        assert_eq!(
            doc.diagnostics,
            vec![Diagnostic::UnresolvedCitation {
                label: "R2".to_string(),
                line: 2,
            }]
        );
        assert_anchor_leading_invariants(&doc.body, "softbreak same para");
    }

    // Unicode surrounding citations
    {
        let input = "[R1] Entry\n\nUnicode quotes: “[R1]”, Japanese: 『[R1]』, Arabic: ﴾[R1]﴿, Emoji: 📚[R1]🎯\n";
        let doc = parse_doc(input);
        assert_eq!(doc.reference_entries.len(), 1);
        assert_eq!(doc.citations.len(), 4);
        for cit in &doc.citations {
            assert_eq!(cit.target, Some("ref-r1".to_string()));
        }
        assert_anchor_leading_invariants(&doc.body, "unicode surrounding");
    }
}

// ---------------------------------------------------------------------------
// 8. Contract-wide invariant validation helper and proptest harnesses
// ---------------------------------------------------------------------------
fn collect_all_anchors(nodes: &[Node], out: &mut Vec<String>) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            Node::Anchor { id } => {
                assert_eq!(i, 0, "Anchor {id} not at index 0 of container");
                out.push(id.clone());
            }
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => {
                collect_all_anchors(children, out);
            }
            Node::List { items, .. } => {
                for item in items {
                    collect_all_anchors(item, out);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_all_anchors(cell, out);
                }
                for row in rows {
                    for cell in row {
                        collect_all_anchors(cell, out);
                    }
                }
            }
            _ => {}
        }
    }
}

fn check_full_contract_invariants(doc: &Document) {
    let mut anchors = Vec::new();
    collect_all_anchors(&doc.body, &mut anchors);

    // Invariant 1: every Node::Anchor belongs to a reference entry
    assert_eq!(
        anchors.len(),
        doc.reference_entries.len(),
        "Number of Node::Anchor in body ({}) must equal reference_entries.len() ({})",
        anchors.len(),
        doc.reference_entries.len()
    );

    // Invariant 2: anchor ids are unique on a page
    let mut sorted_anchors = anchors.clone();
    sorted_anchors.sort();
    let original_len = sorted_anchors.len();
    sorted_anchors.dedup();
    assert_eq!(
        sorted_anchors.len(),
        original_len,
        "Duplicate Node::Anchor ids found in body!"
    );

    for (entry, anchor_id) in doc.reference_entries.iter().zip(&anchors) {
        assert_eq!(&entry.anchor, anchor_id);
        assert_eq!(
            entry.anchor,
            format!("ref-{}", entry.label.to_ascii_lowercase())
        );
    }

    // Invariant 3: every resolved Citation::target is the anchor of exactly one Node::Anchor
    let anchor_set: std::collections::BTreeSet<_> = anchors.iter().cloned().collect();
    for cit in &doc.citations {
        if let Some(target) = &cit.target {
            assert!(
                anchor_set.contains(target),
                "Citation target {target} not found among page anchors!"
            );
        }
    }

    // Invariant 4: if reference_entries is empty, no UnresolvedCitation diagnostic is emitted
    if doc.reference_entries.is_empty() {
        for d in &doc.diagnostics {
            if let Diagnostic::UnresolvedCitation { label, line } = d {
                panic!(
                    "UnresolvedCitation emitted when reference_entries is empty: {label} at line {line}"
                );
            }
        }
        for cit in &doc.citations {
            assert!(
                cit.target.is_none(),
                "Citation {} resolved when page has 0 reference entries",
                cit.label
            );
        }
    }

    // Invariant 5: every DuplicateReferenceEntry diagnostic has first_line <= second_line
    for d in &doc.diagnostics {
        if let Diagnostic::DuplicateReferenceEntry {
            first_line,
            second_line,
            ..
        } = d
        {
            assert!(
                first_line <= second_line,
                "first_line {first_line} > second_line {second_line}"
            );
        }
    }
}

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn prop_random_text_maintains_all_contract_invariants(text in any::<String>()) {
        let doc = parse_doc(&text);
        check_full_contract_invariants(&doc);
    }

    #[test]
    fn prop_bracket_and_citation_stress(
        tokens in prop::collection::vec(
            prop::sample::select(vec![
                "[", "]", "[[", "]]", " ", "\n", "\n\n", "a", "b", "c",
                "[R1]", "[R2]", "[R1a]", "[R99]", "[r1]", "[R1A]", "[R1ab]",
                "[R1] Entry text\n\n",
                "- [R1] List item\n",
                "> [R1] Quote\n\n",
                "[R1]: https://example.com\n\n",
                "# Heading [R1]\n\n",
                "`[R1]`", "```\n[R1]\n```\n\n",
                "[link](http://x)",
                "![img](http://x)",
                "| [R1] |\n|---|\n| [R2] |\n\n",
                "§1", "§1.2",
            ]),
            1..30
        )
    ) {
        let md = tokens.join("");
        let doc = parse_doc(&md);
        check_full_contract_invariants(&doc);
    }
}
