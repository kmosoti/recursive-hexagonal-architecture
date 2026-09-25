//! Registered grader for CHG-013 stage A (front matter and tags, `---`),
//! against the pre-code corpus at `xtask/tests/corpus/front-matter/`. See
//! that package's `README.md` for how the corpus was generated and what it
//! deliberately leaves untested.
//!
//! This grader is written and registered before any implementation exists
//! (decisions `front-matter-first-line`, `front-matter-grammar`,
//! `front-matter-check-surface`, `tags-stage-split`, `front-matter-compat`;
//! `.rha/tasks/CHG-008-growth.toml`). It assumes the following additions to
//! `document`'s public API, exactly as specified by
//! `docs/architecture/front-matter-contract.md` section 3:
//!   - `Document::front_matter: Option<FrontMatter>`
//!   - `FrontMatter { title: Option<String>, tags: Vec<String>, end_line: usize }`
//!   - `Diagnostic::InvalidFrontMatter { line: usize }`
//!
//! None of these exist yet; this file will not compile until they do. A
//! stub crate exercising this exact shape was compiled from the scratch
//! folder as part of writing this grader (see the oracle author's report).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;

use document::{Diagnostic, Document, FrontMatter, Node, parse};
use library::{RelPath, Source};
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

const EXPECTED_CASE_COUNT: usize = 62;

/// The package, embedded at compile time: a core crate's tests do no file
/// I/O (decision pure-fixture-inputs; the core Clippy deny list forbids
/// `std::fs`).
const PACKAGE: &[(&str, &[u8])] = &[
    (
        "SHA256SUMS",
        include_bytes!("../../../xtask/tests/corpus/front-matter/SHA256SUMS"),
    ),
    (
        "CASES.json",
        include_bytes!("../../../xtask/tests/corpus/front-matter/CASES.json"),
    ),
    (
        "README.md",
        include_bytes!("../../../xtask/tests/corpus/front-matter/README.md"),
    ),
    (
        "reference.py",
        include_bytes!("../../../xtask/tests/corpus/front-matter/reference.py"),
    ),
    (
        "selftestreport.txt",
        include_bytes!("../../../xtask/tests/corpus/front-matter/selftestreport.txt"),
    ),
    (
        "source-snapshots/contract.md",
        include_bytes!("../../../xtask/tests/corpus/front-matter/source-snapshots/contract.md"),
    ),
    (
        "source-snapshots/prompt.md",
        include_bytes!("../../../xtask/tests/corpus/front-matter/source-snapshots/prompt.md"),
    ),
];

fn embedded(path: &str) -> &'static [u8] {
    PACKAGE
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| *bytes)
        .unwrap_or_else(|| panic!("{path}: not embedded"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    library::Digest::of(bytes).to_string()
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

fn expected_front_matter(case: &Value, id: &str) -> Option<FrontMatter> {
    let value = &case["front_matter"];
    if value.is_null() {
        return None;
    }
    let title = value["title"].as_str().map(str::to_owned);
    let tags = value["tags"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: front_matter.tags"))
        .iter()
        .map(|t| {
            t.as_str()
                .unwrap_or_else(|| panic!("{id}: front_matter.tags element"))
                .to_owned()
        })
        .collect();
    let end_line = value["end_line"]
        .as_u64()
        .unwrap_or_else(|| panic!("{id}: front_matter.end_line")) as usize;
    Some(FrontMatter {
        title,
        tags,
        end_line,
        index: None,
    })
}

fn expected_invalid_lines(case: &Value, id: &str) -> Vec<usize> {
    case["invalid_front_matter_lines"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing invalid_front_matter_lines array"))
        .iter()
        .map(|v| {
            v.as_u64()
                .unwrap_or_else(|| panic!("{id}: invalid_front_matter_lines element"))
                as usize
        })
        .collect()
}

fn expected_headings(case: &Value, id: &str) -> Vec<(u8, String, usize)> {
    case["headings"]
        .as_array()
        .unwrap_or_else(|| panic!("{id}: missing headings array"))
        .iter()
        .map(|h| {
            let level = h["level"]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: heading level")) as u8;
            let text = h["text"]
                .as_str()
                .unwrap_or_else(|| panic!("{id}: heading text"))
                .to_owned();
            let line = h["line"]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: heading line")) as usize;
            (level, text, line)
        })
        .collect()
}

fn actual_invalid_lines(document: &Document) -> Vec<usize> {
    document
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            Diagnostic::InvalidFrontMatter { line } => Some(*line),
            _ => None,
        })
        .collect()
}

fn actual_headings(document: &Document) -> Vec<(u8, String, usize)> {
    document
        .headings
        .iter()
        .map(|h| (h.level, h.text.clone(), h.line))
        .collect()
}

#[test]
fn all_registered_front_matter_cases_grade_exact_front_matter_title_diagnostics_and_headings() {
    verify_sha256sums();
    let cases = load_cases();
    assert_eq!(
        cases.len(),
        EXPECTED_CASE_COUNT,
        "registered front-matter case count"
    );

    for case in &cases {
        let id = expect_str(case, "id", "<unknown case>").to_owned();
        let markdown = expect_str(case, "markdown", &id).to_owned();

        let rel_path = RelPath::new("fixture.md").expect("fixture relative path");
        let source = Source::new(rel_path, markdown);
        let document = parse(&source);

        // --- Section 3 observation surface: exact match. ---
        assert_eq!(
            document.front_matter,
            expected_front_matter(case, &id),
            "{id}: front_matter"
        );

        assert_eq!(
            document.title,
            expect_str(case, "title", &id),
            "{id}: title"
        );

        assert_eq!(
            actual_invalid_lines(&document),
            expected_invalid_lines(case, &id),
            "{id}: InvalidFrontMatter diagnostic lines, in order"
        );

        assert_eq!(
            actual_headings(&document),
            expected_headings(case, &id),
            "{id}: headings (level, text, line)"
        );

        // --- Node-level invariant (contract section 1, 3): front-matter
        // lines produce no body nodes. The clearest historical regression
        // signal is the pre-feature rendering ("a horizontal rule followed
        // by a heading made of the metadata text", contract "Purpose"): a
        // `Node::Rule` must never be the first body node once front matter
        // is recognized.
        if document.front_matter.is_some() {
            assert!(
                !matches!(document.body.first(), Some(Node::Rule)),
                "{id}: front matter is recognized, so the body must not begin with Node::Rule \
                 (the pre-feature rendering of a delimiter line as a thematic break)"
            );
        }
    }
}
