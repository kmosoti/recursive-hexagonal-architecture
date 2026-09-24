//! Registered grader for CHG-011 (citations, `[Rn]`), against the pre-code
//! corpus at `xtask/tests/corpus/citations/`. See that package's
//! `README.md` for how the corpus was generated and what it deliberately
//! leaves untested.
//!
//! This grader is written and registered before any implementation exists
//! (decisions `citations-anchor-node`, `citations-scope`,
//! `citations-check-surface`; `.rha/tasks/CHG-008-growth.toml`). It assumes
//! the following additions to `document`'s public API, exactly as specified
//! by `docs/architecture/citation-contract.md` section 3-4:
//!   - `Document::reference_entries: Vec<ReferenceEntry>`
//!   - `Document::citations: Vec<Citation>`
//!   - `ReferenceEntry { label: String, anchor: String, line: usize }`
//!   - `Citation { label: String, target: Option<String>, line: usize }`
//!   - `Diagnostic::DuplicateReferenceEntry { label, first_line, second_line }`
//!   - `Diagnostic::UnresolvedCitation { label, line }`
//!   - `Node::Anchor { id: String }`
//! None of these exist yet; this file will not compile until they do. A
//! stub crate exercising this exact shape was compiled from the scratch
//! folder as part of writing this grader (see the oracle author's report).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use document::{Citation, Diagnostic, Document, Node, ReferenceEntry, parse};
use library::{Digest, RelPath, Source};
use serde_json::Value;

/// Payload files covered by `SHA256SUMS`, in the order that file lists them.
const PAYLOAD_FILES: &[&str] = &[
    "CASES.json",
    "README.md",
    "reference.py",
    "selftestreport.txt",
    "source-snapshots/contract.md",
    "source-snapshots/section-reference-contract.md",
    "source-snapshots/prompt.md",
];

const EXPECTED_CASE_COUNT: usize = 69;

/// The package and the spec, embedded at compile time: a core crate's tests do no file I/O
/// (decision pure-fixture-inputs; the core Clippy deny list forbids `std::fs`).
const PACKAGE: &[(&str, &[u8])] = &[
    (
        "SHA256SUMS",
        include_bytes!("../../../xtask/tests/corpus/citations/SHA256SUMS"),
    ),
    (
        "CASES.json",
        include_bytes!("../../../xtask/tests/corpus/citations/CASES.json"),
    ),
    (
        "README.md",
        include_bytes!("../../../xtask/tests/corpus/citations/README.md"),
    ),
    (
        "reference.py",
        include_bytes!("../../../xtask/tests/corpus/citations/reference.py"),
    ),
    (
        "selftestreport.txt",
        include_bytes!("../../../xtask/tests/corpus/citations/selftestreport.txt"),
    ),
    (
        "source-snapshots/contract.md",
        include_bytes!("../../../xtask/tests/corpus/citations/source-snapshots/contract.md"),
    ),
    (
        "source-snapshots/section-reference-contract.md",
        include_bytes!(
            "../../../xtask/tests/corpus/citations/source-snapshots/section-reference-contract.md"
        ),
    ),
    (
        "source-snapshots/prompt.md",
        include_bytes!("../../../xtask/tests/corpus/citations/source-snapshots/prompt.md"),
    ),
];

const SPEC_PATH: &str = "docs/spec/rha-spec-v0.10.md";
const SPEC: &[u8] = include_bytes!("../../../docs/spec/rha-spec-v0.10.md");

fn embedded(path: &str) -> &'static [u8] {
    PACKAGE
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| *bytes)
        .unwrap_or_else(|| panic!("{path}: not embedded"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Digest::of(bytes).to_string()
}

/// Verify every payload file's digest against `SHA256SUMS` before trusting
/// anything else in the package.
fn verify_sha256sums() {
    let sums_text = std::str::from_utf8(embedded("SHA256SUMS")).expect("SHA256SUMS is UTF-8");

    let mut listed = BTreeMap::new();
    for line in sums_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, "  ");
        let digest = parts.next().expect("digest column");
        let path = parts
            .next()
            .unwrap_or_else(|| panic!("malformed SHA256SUMS line: {line}"));
        listed.insert(path.to_owned(), digest.to_owned());
    }

    let expected_paths = PAYLOAD_FILES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    let mut listed_paths = listed.keys().cloned().collect::<Vec<_>>();
    listed_paths.sort();
    let mut expected_sorted = expected_paths.clone();
    expected_sorted.sort();
    assert_eq!(
        listed_paths, expected_sorted,
        "SHA256SUMS payload file list"
    );

    for path in PAYLOAD_FILES {
        let actual = sha256_hex(embedded(path));
        let expected = listed
            .get(*path)
            .unwrap_or_else(|| panic!("{path}: not listed in SHA256SUMS"));
        assert_eq!(
            &actual, expected,
            "{path}: sha256 mismatch against SHA256SUMS"
        );
    }
}

fn load_cases() -> Vec<Value> {
    let value: Value =
        serde_json::from_slice(embedded("CASES.json")).expect("CASES.json is valid JSON");
    value
        .as_array()
        .expect("CASES.json is a JSON array")
        .clone()
}

fn expect_str<'a>(case: &'a Value, key: &str, id: &str) -> &'a str {
    case[key]
        .as_str()
        .unwrap_or_else(|| panic!("{id}: missing string field {key}"))
}

fn case_markdown(case: &Value, id: &str) -> String {
    if let Some(markdown) = case.get("markdown").and_then(Value::as_str) {
        return markdown.to_owned();
    }

    // The real-spec case: identified by path + sha256 instead of inlined text.
    let path = expect_str(case, "path", id);
    let expected_sha256 = expect_str(case, "sha256", id);
    assert_eq!(
        path, SPEC_PATH,
        "{id}: the only path-identified case is the spec"
    );
    let bytes = SPEC.to_vec();
    let actual_sha256 = sha256_hex(&bytes);
    assert_eq!(
        actual_sha256, expected_sha256,
        "{id}: {path} sha256 mismatch against registration"
    );
    String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("{id}: {path} is not valid UTF-8: {error}"))
}

fn expected_entries(case: &Value, id: &str) -> Vec<ReferenceEntry> {
    case["reference_entries"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing reference_entries array"))
        .iter()
        .map(|entry| ReferenceEntry {
            label: expect_str(entry, "label", id).to_owned(),
            anchor: expect_str(entry, "anchor", id).to_owned(),
            line: entry["line"]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: entry line")) as usize,
        })
        .collect()
}

fn expected_citations(case: &Value, id: &str) -> Vec<Citation> {
    case["citations"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing citations array"))
        .iter()
        .map(|entry| Citation {
            label: expect_str(entry, "label", id).to_owned(),
            target: entry["target"].as_str().map(str::to_owned),
            line: entry["line"]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: citation line")) as usize,
        })
        .collect()
}

fn expected_duplicate_diagnostics(case: &Value, id: &str) -> Vec<(String, usize, usize)> {
    let mut out = case["diagnostics"]["duplicate_reference_entry"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing diagnostics.duplicate_reference_entry array"))
        .iter()
        .map(|entry| {
            (
                expect_str(entry, "label", id).to_owned(),
                entry["first_line"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{id}: first_line")) as usize,
                entry["second_line"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{id}: second_line")) as usize,
            )
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn expected_unresolved_diagnostics(case: &Value, id: &str) -> Vec<(String, usize)> {
    let mut out = case["diagnostics"]["unresolved_citation"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing diagnostics.unresolved_citation array"))
        .iter()
        .map(|entry| {
            (
                expect_str(entry, "label", id).to_owned(),
                entry["line"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{id}: line")) as usize,
            )
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn actual_duplicate_diagnostics(document: &Document) -> Vec<(String, usize, usize)> {
    let mut out = document
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            Diagnostic::DuplicateReferenceEntry {
                label,
                first_line,
                second_line,
            } => Some((label.clone(), *first_line, *second_line)),
            _ => None,
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn actual_unresolved_diagnostics(document: &Document) -> Vec<(String, usize)> {
    let mut out = document
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            Diagnostic::UnresolvedCitation { label, line } => Some((label.clone(), *line)),
            _ => None,
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

/// The concatenation of a node list's visible plain text (sufficient for a
/// resolved citation's link text, which is never itself styled).
fn plain_text(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(text) | Node::Code(text) | Node::Html(text) => out.push_str(text),
            Node::Emphasis(children) | Node::Strong(children) | Node::Strikethrough(children) => {
                out.push_str(&plain_text(children));
            }
            Node::SoftBreak => out.push(' '),
            Node::HardBreak => out.push('\n'),
            other => panic!("unexpected node inside a citation link's visible text: {other:?}"),
        }
    }
    out
}

/// Every `Node::Link` in the body, as `(href, visible text)`, at any depth.
fn collect_links(nodes: &[Node], out: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            Node::Link { href, children } => {
                out.push((href.clone(), plain_text(children)));
                collect_links(children, out);
            }
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => collect_links(children, out),
            Node::List { items, .. } => {
                for item in items {
                    collect_links(item, out);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_links(cell, out);
                }
                for row in rows {
                    for cell in row {
                        collect_links(cell, out);
                    }
                }
            }
            Node::Anchor { .. }
            | Node::Text(_)
            | Node::Code(_)
            | Node::CodeBlock { .. }
            | Node::Image { .. }
            | Node::Rule
            | Node::SoftBreak
            | Node::HardBreak
            | Node::Html(_)
            | Node::TaskMarker(_)
            | Node::Transclusion(_) => {}
        }
    }
}

/// Every `Node::Anchor { id }` in the body, and its position within the
/// immediate `Vec<Node>` that directly contains it. The contract says the
/// entry paragraph "gains `Node::Anchor { id }` as its first inline node"
/// (section 1); this package does not assume whether a list item's or a
/// block quote's paragraph content is wrapped in an explicit
/// `Node::Paragraph` (an implementation-shape question this package leaves
/// open, see `README.md`), so it checks the weaker, tree-shape-tolerant
/// form of the same invariant: wherever a `Node::Anchor` occurs, it is the
/// first element of whichever `Vec<Node>` directly holds it (a paragraph's
/// children, a list item's content, a block quote's children, or a table
/// cell), and never any other position in that vector.
fn collect_anchors_checking_leading_position(nodes: &[Node], id: &str, out: &mut Vec<String>) {
    for (index, node) in nodes.iter().enumerate() {
        match node {
            Node::Anchor { id: anchor_id } => {
                assert_eq!(
                    index, 0,
                    "{id}: Node::Anchor {{ id: {anchor_id:?} }} must be the first inline node of its containing paragraph"
                );
                out.push(anchor_id.clone());
            }
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. }
            | Node::Link { children, .. } => {
                collect_anchors_checking_leading_position(children, id, out);
            }
            Node::List { items, .. } => {
                for item in items {
                    collect_anchors_checking_leading_position(item, id, out);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_anchors_checking_leading_position(cell, id, out);
                }
                for row in rows {
                    for cell in row {
                        collect_anchors_checking_leading_position(cell, id, out);
                    }
                }
            }
            Node::Text(_)
            | Node::Code(_)
            | Node::CodeBlock { .. }
            | Node::Image { .. }
            | Node::Rule
            | Node::SoftBreak
            | Node::HardBreak
            | Node::Html(_)
            | Node::TaskMarker(_)
            | Node::Transclusion(_) => {}
        }
    }
}

#[test]
fn all_registered_citation_cases_grade_exact_entries_citations_diagnostics_and_node_invariants() {
    verify_sha256sums();
    let cases = load_cases();
    assert_eq!(
        cases.len(),
        EXPECTED_CASE_COUNT,
        "registered citations case count"
    );

    for case in &cases {
        let id = expect_str(case, "id", "<unknown case>").to_owned();
        let markdown = case_markdown(case, &id);

        let rel_path = RelPath::new("fixture.md").expect("fixture relative path");
        let source = Source::new(rel_path, markdown);
        let document = parse(&source);

        // --- Section 3 observation surface: exact match. ---
        let expected_entries_vec = expected_entries(case, &id);
        assert_eq!(
            document.reference_entries, expected_entries_vec,
            "{id}: reference_entries"
        );

        let expected_citations_vec = expected_citations(case, &id);
        assert_eq!(
            document.citations, expected_citations_vec,
            "{id}: citations"
        );

        let expected_dup = expected_duplicate_diagnostics(case, &id);
        let actual_dup = actual_duplicate_diagnostics(&document);
        assert_eq!(
            actual_dup, expected_dup,
            "{id}: DuplicateReferenceEntry diagnostics"
        );

        let expected_unresolved = expected_unresolved_diagnostics(case, &id);
        let actual_unresolved = actual_unresolved_diagnostics(&document);
        assert_eq!(
            actual_unresolved, expected_unresolved,
            "{id}: UnresolvedCitation diagnostics"
        );

        // --- Node-level invariants (contract section 1, 2, 5). ---

        // Every resolved citation has a Node::Link with href "#"+anchor and
        // text exactly the citation's characters ("[R<label>]").
        let mut expected_link_pairs = expected_citations_vec
            .iter()
            .filter_map(|citation| {
                citation
                    .target
                    .as_ref()
                    .map(|anchor| (format!("#{anchor}"), format!("[{}]", citation.label)))
            })
            .collect::<Vec<_>>();
        expected_link_pairs.sort();

        let mut actual_links = Vec::new();
        collect_links(&document.body, &mut actual_links);
        let mut actual_hash_links = actual_links
            .into_iter()
            .filter(|(href, _)| href.starts_with("#ref-"))
            .collect::<Vec<_>>();
        actual_hash_links.sort();

        assert_eq!(
            actual_hash_links, expected_link_pairs,
            "{id}: citation links must match resolved citations exactly, one Node::Link per resolved citation"
        );

        // Every Node::Anchor belongs to a reference entry, is the first
        // inline node of whichever Vec<Node> directly contains it, and
        // anchor ids are unique (contract section 1, 5).
        let mut all_anchor_ids = Vec::new();
        collect_anchors_checking_leading_position(&document.body, &id, &mut all_anchor_ids);

        let mut expected_anchor_ids = expected_entries_vec
            .iter()
            .map(|entry| entry.anchor.clone())
            .collect::<Vec<_>>();
        let mut actual_anchor_ids_sorted = all_anchor_ids.clone();
        expected_anchor_ids.sort();
        actual_anchor_ids_sorted.sort();
        assert_eq!(
            actual_anchor_ids_sorted, expected_anchor_ids,
            "{id}: every Node::Anchor belongs to exactly one reference entry, and vice versa"
        );

        let mut unique_check = all_anchor_ids.clone();
        unique_check.sort();
        unique_check.dedup();
        assert_eq!(
            unique_check.len(),
            all_anchor_ids.len(),
            "{id}: anchor ids are unique"
        );
    }
}
