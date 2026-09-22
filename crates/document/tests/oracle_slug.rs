#![allow(clippy::unwrap_used, clippy::expect_used)]

use document::{parse, slugify};
use library::{RelPath, Source};
use proptest::prelude::*;
use std::collections::BTreeMap;

// CHG-005 (P-A): the one edit the implementing session made to this
// independent oracle. Its author was given the W5 brief's NFC step; the
// registered markdown contract, which grades the product, has none (decision
// slug-without-nfc). Everything else is as written.
fn ref_slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .filter_map(|character| match character {
            character if character.is_alphanumeric() => Some(character),
            ' ' => Some('-'),
            '-' | '_' => Some(character),
            _ => None,
        })
        .collect()
}

fn edge_fragment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("\u{0301}".to_owned()),
        Just("\u{0307}".to_owned()),
        Just("\u{0308}".to_owned()),
        Just("e\u{0301}".to_owned()),
        Just("é".to_owned()),
        Just("A\u{030A}".to_owned()),
        Just("Å".to_owned()),
        Just("İ".to_owned()),
        Just("ß".to_owned()),
        Just("Σ".to_owned()),
        Just("σ".to_owned()),
        Just("ς".to_owned()),
        Just("ΟΣ".to_owned()),
        Just("١२０".to_owned()),
        Just(".,!?;:()[]{}*".to_owned()),
        Just("A!B".to_owned()),
        Just("___".to_owned()),
        Just("---".to_owned()),
        Just("  ".to_owned()),
        Just("\t".to_owned()),
        any::<char>().prop_map(|character| character.to_string()),
    ]
}

proptest! {
    #[test]
    fn slugify_matches_independent_reference_for_arbitrary_strings(input in any::<String>()) {
        prop_assert_eq!(slugify(&input), ref_slugify(&input));
    }

    #[test]
    fn slugify_matches_independent_reference_for_unicode_edges(
        pieces in prop::collection::vec(edge_fragment(), 0..64)
    ) {
        let input = pieces.concat();
        prop_assert_eq!(slugify(&input), ref_slugify(&input));
    }
}

fn heading_text() -> impl Strategy<Value = String> {
    // Restricted letters, digits, spaces, and punctuation cannot introduce
    // Markdown syntax; H/Z boundaries avoid whitespace trimming.
    "[A-Za-z0-9éÉßΣσςЖж中١२０ .,!?;:()]{0,24}".prop_map(|middle| format!("H{middle}Z"))
}

proptest! {
    #[test]
    fn parsed_heading_slugs_match_reference_and_occurrence_suffixes(
        pool in prop::collection::vec(heading_text(), 1..8),
        selections in prop::collection::vec(0usize..8, 0..25),
    ) {
        let headings: Vec<String> = selections
            .iter()
            .map(|index| pool[*index % pool.len()].clone())
            .collect();
        let markdown = headings
            .iter()
            .map(|heading| format!("## {heading}\n\n"))
            .collect::<String>();
        let source = Source::new(RelPath::new("p.md").unwrap(), markdown);
        let document = parse(&source);

        let mut occurrences = BTreeMap::<String, usize>::new();
        let expected: Vec<String> = headings
            .iter()
            .map(|heading| {
                let base = ref_slugify(heading);
                let count = occurrences.entry(base.clone()).or_default();
                *count += 1;
                if *count == 1 {
                    base
                } else {
                    format!("{base}-{}", *count)
                }
            })
            .collect();
        let actual: Vec<String> = document
            .headings
            .iter()
            .map(|heading| heading.slug.clone())
            .collect();

        prop_assert_eq!(actual, expected);
    }
}
