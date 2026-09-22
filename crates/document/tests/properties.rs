//! Plan §3.2: slugs unique or witnessed; parsing is total.

use document::{Diagnostic, parse};
use library::{RelPath, Source};
use proptest::prelude::*;

fn source(text: &str) -> Source {
    Source::new(RelPath::new("p.md").expect("valid"), text.to_owned())
}

proptest! {
    #[test]
    fn parsing_is_total(text in any::<String>()) {
        let _ = parse(&source(&text));
    }

    #[test]
    fn final_slugs_are_unique_and_every_repeat_is_witnessed(titles in prop::collection::vec(prop::sample::select(vec!["A", "a", "A!", "b c", "B  C", "x"]), 0..10)) {
        let text: String = titles.iter().map(|t| format!("## {t}\n\n")).collect();
        let doc = parse(&source(&text));
        let mut slugs: Vec<_> = doc.headings.iter().map(|h| h.slug.clone()).collect();
        let n = slugs.len();
        slugs.sort();
        slugs.dedup();
        prop_assert_eq!(slugs.len(), n);
        let mut bases: Vec<_> = doc.headings.iter().map(|h| h.base_slug.clone()).collect();
        let total = bases.len();
        bases.sort();
        bases.dedup();
        let dups = doc.diagnostics.iter().filter(|d| matches!(d, Diagnostic::DuplicateSlug { .. })).count();
        prop_assert_eq!(dups, total - bases.len());
    }
}

#[test]
fn wikilinks_split_target_anchor_and_alias_and_ignore_code() {
    let doc = parse(&source(
        "See [[a/b#Sec One|al *em*]] and [[#local]] and `[[not]]`.\n\n```\n[[fenced]]\n```\n",
    ));
    let links: Vec<_> = doc
        .links
        .iter()
        .map(|l| (l.target.as_str(), l.anchor.as_deref(), l.alias.as_deref()))
        .collect();
    assert_eq!(
        links,
        vec![
            ("a/b", Some("Sec One"), Some("al em")),
            ("", Some("local"), None)
        ]
    );
}

#[test]
fn heading_text_keeps_code_and_link_text() {
    let doc = parse(&source(
        "# Head `code` [x](y) *em*\n# Head `code` [x](y) *em*\n",
    ));
    assert_eq!(doc.headings[0].slug, "head-code-x-em");
    assert_eq!(doc.headings[1].slug, "head-code-x-em-2");
    assert_eq!(doc.title, "Head code x em");
}

#[test]
fn a_generated_suffix_never_collides_with_a_natural_slug() {
    let doc = parse(&source("# A\n# A\n# A-2\n# A-2\n"));
    let slugs: Vec<&str> = doc.headings.iter().map(|h| h.slug.as_str()).collect();
    assert_eq!(slugs, ["a", "a-2", "a-2-2", "a-2-3"]);
    let witnesses = doc
        .diagnostics
        .iter()
        .filter(|d| matches!(d, Diagnostic::DuplicateSlug { .. }))
        .count();
    assert_eq!(witnesses, 2, "one per repeated base: a once, a-2 once");
}
