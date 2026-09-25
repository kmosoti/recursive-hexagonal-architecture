//! "Nothing else changes" for CHG-011 (citation contract §5): a page with no citation and no
//! reference entry parses exactly as before the feature. The expected trees were captured with
//! the parser at e2a2f95, the commit before any citation code, by the supervising session (a
//! different model family from the implementer): every docs page without `[R`, plus
//! bracket-heavy inputs, since `[` and `]` split pulldown-cmark text events.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use document::parse;
use library::{RelPath, Source};

#[test]
fn pages_without_citations_parse_exactly_as_before() {
    let golden: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(include_str!("data/pre_citation_trees.json")).expect("golden trees");
    assert_eq!(golden.len(), 44);
    for (name, case) in golden {
        let markdown = case["markdown"].as_str().expect("markdown");
        let document = parse(&Source::new(
            RelPath::new("p.md").expect("path"),
            markdown.to_owned(),
        ));
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
        assert!(document.citations.is_empty(), "{name}: citations");
        assert!(document.reference_entries.is_empty(), "{name}: entries");
    }
}
