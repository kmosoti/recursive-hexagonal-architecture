//! Registered grader for CHG-010 (`§`-references), against the pre-code
//! corpus at `xtask/tests/corpus/section-refs/`. See that package's
//! `README.md` for how the corpus was generated and what it deliberately
//! leaves untested.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use document::{Diagnostic, Document, Node, SectionRef, parse};
use library::{Digest, RelPath, Source};
use serde_json::Value;

/// Payload files covered by `SHA256SUMS`, in the order that file lists them.
const PAYLOAD_FILES: &[&str] = &[
    "CASES.json",
    "README.md",
    "reference.py",
    "selftestreport.txt",
    "source-snapshots/contract.md",
    "source-snapshots/prompt.md",
];

const EXPECTED_CASE_COUNT: usize = 84;

/// The package and the spec, embedded at compile time: a core crate's tests do no file I/O
/// (decision pure-fixture-inputs; the core Clippy deny list forbids `std::fs`).
const PACKAGE: &[(&str, &[u8])] = &[
    (
        "SHA256SUMS",
        include_bytes!("../../../xtask/tests/corpus/section-refs/SHA256SUMS"),
    ),
    (
        "CASES.json",
        include_bytes!("../../../xtask/tests/corpus/section-refs/CASES.json"),
    ),
    (
        "README.md",
        include_bytes!("../../../xtask/tests/corpus/section-refs/README.md"),
    ),
    (
        "reference.py",
        include_bytes!("../../../xtask/tests/corpus/section-refs/reference.py"),
    ),
    (
        "selftestreport.txt",
        include_bytes!("../../../xtask/tests/corpus/section-refs/selftestreport.txt"),
    ),
    (
        "source-snapshots/contract.md",
        include_bytes!("../../../xtask/tests/corpus/section-refs/source-snapshots/contract.md"),
    ),
    (
        "source-snapshots/prompt.md",
        include_bytes!("../../../xtask/tests/corpus/section-refs/source-snapshots/prompt.md"),
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

fn expected_refs(case: &Value, id: &str, document: &Document) -> Vec<SectionRef> {
    case["refs"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing refs array"))
        .iter()
        .map(|entry| {
            let text = expect_str(entry, "text", id).to_owned();
            let number = expect_str(entry, "number", id).to_owned();
            let line: usize = entry["line"]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: ref line")) as usize;
            let target = match &entry["target_heading_line"] {
                Value::Null => None,
                value => {
                    let target_line = value
                        .as_u64()
                        .unwrap_or_else(|| panic!("{id}: target_heading_line"))
                        as usize;
                    let heading = document
                        .headings
                        .iter()
                        .find(|heading| heading.line == target_line)
                        .unwrap_or_else(|| {
                            panic!("{id}: no heading found at registered target line {target_line}")
                        });
                    Some(heading.slug.clone())
                }
            };
            SectionRef {
                text,
                number,
                target,
                line,
            }
        })
        .collect()
}

fn expected_diagnostics(case: &Value, id: &str) -> Vec<(String, usize)> {
    let mut out = case["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing diagnostics array"))
        .iter()
        .map(|entry| {
            let number = expect_str(entry, "number", id).to_owned();
            let line: usize = entry["line"]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: diagnostic line"))
                as usize;
            (number, line)
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn actual_diagnostics(document: &Document) -> Vec<(String, usize)> {
    let mut out = document
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            Diagnostic::UnresolvedSectionRef { number, line } => Some((number.clone(), *line)),
            _ => None,
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

/// The concatenation of a node list's visible plain text (sufficient for
/// this corpus's section-ref link text, which is never itself styled).
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
            other => panic!("unexpected node inside a section-ref link's visible text: {other:?}"),
        }
    }
    out
}

/// A citation link (CHG-011, citation contract section 2): visible text
/// `[R<digits>]` with at most one lowercase letter, linking to
/// `#ref-<label lowercase>`. Section references never have this shape (their
/// text begins with `§`), so excluding exactly these keeps every other
/// intra-page link under this grader's count.
fn is_citation_link(href: &str, text: &str) -> bool {
    let Some(label) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) else {
        return false;
    };
    let Some(rest) = label.strip_prefix('R') else {
        return false;
    };
    let digits = rest.trim_end_matches(|c: char| c.is_ascii_lowercase());
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && rest.len() - digits.len() <= 1
        && href == format!("#ref-{}", label.to_lowercase())
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
fn all_registered_section_ref_cases_grade_exact_refs_diagnostics_and_link_invariant() {
    verify_sha256sums();
    let cases = load_cases();
    assert_eq!(
        cases.len(),
        EXPECTED_CASE_COUNT,
        "registered section-refs case count"
    );

    for case in &cases {
        let id = expect_str(case, "id", "<unknown case>").to_owned();
        let markdown = case_markdown(case, &id);

        let rel_path = RelPath::new("fixture.md").expect("fixture relative path");
        let source = Source::new(rel_path, markdown);
        let document = parse(&source);

        let expected = expected_refs(case, &id, &document);
        assert_eq!(document.section_refs, expected, "{id}: section_refs");

        let expected_diag = expected_diagnostics(case, &id);
        let actual_diag = actual_diagnostics(&document);
        assert_eq!(
            actual_diag, expected_diag,
            "{id}: UnresolvedSectionRef diagnostics"
        );

        // Invariant: every resolved ref has a matching `Node::Link`, and the
        // number of links shaped like a section-ref link equals the number
        // of resolved refs (no missing and no spurious link).
        let mut expected_link_pairs = expected
            .iter()
            .filter_map(|section_ref| {
                section_ref
                    .target
                    .as_ref()
                    .map(|slug| (format!("#{slug}"), section_ref.text.clone()))
            })
            .collect::<Vec<_>>();
        expected_link_pairs.sort();

        let mut actual_links = Vec::new();
        collect_links(&document.body, &mut actual_links);
        let mut actual_hash_links = actual_links
            .into_iter()
            .filter(|(href, text)| href.starts_with('#') && !is_citation_link(href, text))
            .collect::<Vec<_>>();
        actual_hash_links.sort();

        assert_eq!(
            actual_hash_links, expected_link_pairs,
            "{id}: section-ref links must match resolved refs exactly, one Node::Link per resolved ref"
        );
    }
}
