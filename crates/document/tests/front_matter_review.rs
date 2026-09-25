//! Regression tests from the independent review of 9a2e44e (CHG-013 stage A), written before the
//! repair. Contract section 2.1, item 5: a carriage return at the end of a line counts as trailing
//! whitespace for the delimiters AND for every front-matter line. The implementation trimmed any
//! mix of spaces, tabs and CRs from delimiters but stripped only one final CR from front-matter
//! lines.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::{Diagnostic, parse};
use library::{RelPath, Source};

fn doc(text: &str) -> document::Document {
    parse(&Source::new(
        RelPath::new("p.md").expect("path"),
        text.to_owned(),
    ))
}

fn invalid_lines(d: &document::Document) -> Vec<usize> {
    d.diagnostics
        .iter()
        .filter_map(|x| match x {
            Diagnostic::InvalidFrontMatter { line } => Some(*line),
            _ => None,
        })
        .collect()
}

#[test]
fn carriage_return_before_trailing_space_is_trailing_whitespace_in_a_tags_line() {
    let d = doc("---\ntags: [a]\r \n---\n");
    let fm = d.front_matter.clone().expect("front matter");
    assert_eq!(fm.tags, vec!["a".to_owned()]);
    assert!(invalid_lines(&d).is_empty(), "{:?}", d.diagnostics);
}

#[test]
fn carriage_returns_and_spaces_after_a_title_are_trailing_whitespace() {
    for text in ["---\ntitle: x\r \n---\n", "---\ntitle: x\r\r\n---\n"] {
        let d = doc(text);
        assert_eq!(
            d.front_matter
                .clone()
                .expect("front matter")
                .title
                .as_deref(),
            Some("x"),
            "{text:?}"
        );
        assert_eq!(d.title, "x", "{text:?}");
    }
}

#[test]
fn a_line_of_only_spaces_and_carriage_returns_is_blank() {
    let d = doc("---\n \r\r\n---\n");
    assert!(d.front_matter.is_some());
    assert!(invalid_lines(&d).is_empty(), "{:?}", d.diagnostics);
}

#[test]
fn a_dash_followed_only_by_a_tab_is_an_empty_list_item() {
    // Section 2.1 items 4 and 5: trailing whitespace is ignored, so ` -\t` is ` -`, an empty tag
    // that is dropped without a diagnostic (a behaviour the repair changed; pinned here).
    for text in ["---\ntags:\n -\t\n---\n", "---\ntags:\n -\t\r\n---\n"] {
        let d = doc(text);
        assert!(
            invalid_lines(&d).is_empty(),
            "{text:?}: {:?}",
            d.diagnostics
        );
        assert!(
            d.front_matter
                .clone()
                .expect("front matter")
                .tags
                .is_empty(),
            "{text:?}"
        );
    }
}
