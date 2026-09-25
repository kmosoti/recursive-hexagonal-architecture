#![allow(clippy::unwrap_used, clippy::expect_used)]

use document::{Document, parse};
use graph::{Witness, resolve};
use library::{RelPath, Source};
use proptest::prelude::*;

#[derive(Clone, Debug)]
struct ReferencePage {
    id: String,
    final_slugs: Vec<String>,
    links: Vec<ReferenceLink>,
}

#[derive(Clone, Debug)]
struct ReferenceLink {
    target: String,
    anchor: Option<String>,
}

type WitnessTuple = (&'static str, String, String, Option<String>);

fn ref_resolve(pages: &[ReferencePage]) -> Vec<WitnessTuple> {
    let mut witnesses = Vec::new();
    for page in pages {
        for link in &page.links {
            let target_lower = link.target.to_lowercase();
            let exact = if link.target.is_empty() {
                Some(page)
            } else {
                pages.iter().find(|candidate| candidate.id == link.target)
            };
            let resolved = if let Some(exact) = exact {
                Some(exact)
            } else {
                let basename_matches: Vec<&ReferencePage> = pages
                    .iter()
                    .filter(|candidate| {
                        candidate
                            .id
                            .rsplit('/')
                            .next()
                            .unwrap_or(&candidate.id)
                            .to_lowercase()
                            == target_lower
                    })
                    .collect();
                match basename_matches.as_slice() {
                    [only] => Some(*only),
                    [] => {
                        witnesses.push(("broken_link", page.id.clone(), link.target.clone(), None));
                        None
                    }
                    _ => {
                        witnesses.push((
                            "ambiguous_link",
                            page.id.clone(),
                            link.target.clone(),
                            None,
                        ));
                        None
                    }
                }
            };

            let Some(target_page) = resolved else {
                continue;
            };
            let Some(anchor) = &link.anchor else {
                continue;
            };
            if !target_page.final_slugs.contains(anchor) {
                witnesses.push((
                    "missing_anchor",
                    page.id.clone(),
                    link.target.clone(),
                    Some(anchor.clone()),
                ));
            }
        }
    }
    witnesses
}

fn case_changed(value: &str) -> String {
    let upper = value.to_uppercase();
    if upper == value {
        value.to_lowercase()
    } else {
        upper
    }
}

fn render_page(headings: &[String], links: &[ReferenceLink]) -> String {
    let mut sections: Vec<String> = headings
        .iter()
        .map(|heading| format!("## {heading}"))
        .collect();
    sections.extend(links.iter().map(|link| match &link.anchor {
        Some(anchor) => format!("[[{}#{}]]", link.target, anchor),
        None => format!("[[{}]]", link.target),
    }));
    sections.join("\n\n")
}

fn project_witness(witness: &Witness) -> WitnessTuple {
    match witness {
        Witness::BrokenLink { from, target, .. } => (
            "broken_link",
            from.as_str().to_owned(),
            target.clone(),
            None,
        ),
        Witness::AmbiguousLink { from, target, .. } => (
            "ambiguous_link",
            from.as_str().to_owned(),
            target.clone(),
            None,
        ),
        Witness::MissingAnchor {
            from,
            target,
            heading,
            ..
        } => (
            "missing_anchor",
            from.as_str().to_owned(),
            target.clone(),
            Some(heading.clone()),
        ),
        Witness::BrokenTransclusion { .. } => {
            panic!("unexpected witness: BrokenTransclusion")
        }
        Witness::AmbiguousTransclusion { .. } => {
            panic!("unexpected witness: AmbiguousTransclusion")
        }
        Witness::MissingTransclusionAnchor { .. } => {
            panic!("unexpected witness: MissingTransclusionAnchor")
        }
        Witness::TransclusionCycle { .. } => panic!("unexpected witness: TransclusionCycle"),
    }
}

fn headings_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("Intro".to_owned()),
            Just("More".to_owned()),
            Just("Repeat".to_owned()),
            Just("Café".to_owned()),
        ],
        0..7,
    )
}

fn anchor_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        1 => Just(None),
        3 => prop_oneof![
            Just("intro".to_owned()),
            Just("intro-2".to_owned()),
            Just("more".to_owned()),
            Just("repeat".to_owned()),
            Just("repeat-2".to_owned()),
            Just("café".to_owned()),
            Just("absent-anchor".to_owned()),
            Just("INTRO".to_owned()),
        ].prop_map(Some),
    ]
}

fn link_spec(
    target_index: usize,
    mode: u8,
    anchor: Option<String>,
    ids: &[String],
    source_index: usize,
) -> ReferenceLink {
    let destination = target_index % ids.len();
    let destination_id = &ids[destination];
    let basename = destination_id.rsplit('/').next().unwrap_or(destination_id);
    let target = match mode {
        0 => destination_id.clone(),                          // Exact whole ID.
        1 => case_changed(basename),                          // Case-insensitive basename fallback.
        2 => format!("missing-{destination}-{source_index}"), // Guaranteed absent.
        3 => String::new(), // Empty target resolves to the source page.
        4 => case_changed(destination_id), // Full ID case changes cannot strip directories.
        5 => basename.to_owned(), // Exact page IDs take priority over basename matches.
        6 => format!("absent-dir-{source_index}/{basename}"), // Directory must not be discarded.
        _ => destination_id.clone(),
    };
    let anchor = if mode == 3 {
        Some(anchor.unwrap_or_else(|| "intro".to_owned()))
    } else {
        anchor
    };
    ReferenceLink { target, anchor }
}

proptest! {
    #[test]
    fn resolver_witnesses_match_independent_reference(
        ids in prop::collection::vec(
            prop_oneof![
                Just("Home".to_owned()),
                Just("home".to_owned()),
                Just("Guide".to_owned()),
                Just("docs/Guide".to_owned()),
                Just("notes/guide".to_owned()),
                Just("API".to_owned()),
                Just("api".to_owned()),
                Just("docs/API".to_owned()),
                Just("Café".to_owned()),
                Just("docs/Café".to_owned()),
                Just("notes/café".to_owned()),
            ],
            0..7,
        ),
        headings_by_page in prop::collection::vec(headings_strategy(), 0..7),
        link_specs in prop::collection::vec(
            prop::collection::vec(
                (any::<usize>(), 0u8..7, anchor_strategy(), 1usize..4),
                0..11,
            ),
            0..7,
        ),
    ) {
        let mut unique_ids = Vec::new();
        for id in ids {
            if !unique_ids.contains(&id) {
                unique_ids.push(id);
            }
        }
        let mut parsed_documents: Vec<Document> = Vec::new();
        let mut reference_pages: Vec<ReferencePage> = Vec::new();
        for (source_index, id) in unique_ids.iter().enumerate() {
            let headings = headings_by_page
                .get(source_index)
                .cloned()
                .unwrap_or_default();
            let specs = link_specs.get(source_index).cloned().unwrap_or_default();
            let mut links = Vec::new();
            for (target_index, mode, anchor, copies) in specs {
                let link = link_spec(target_index, mode, anchor, &unique_ids, source_index);
                for _ in 0..copies {
                    links.push(link.clone());
                }
            }

            let mut markdown_headings = vec!["Intro".to_owned()];
            markdown_headings.extend(headings);
            let source = Source::new(
                RelPath::new(format!("{id}.md").as_str()).unwrap(),
                render_page(&markdown_headings, &links),
            );
            let document = parse(&source);
            // Resolver reference treats parsed Document output as its input contract;
            // slug independence is tested separately.
            let parsed_headings = document
                .headings
                .iter()
                .map(|heading| heading.slug.clone())
                .collect();
            let parsed_links = document
                .links
                .iter()
                .map(|link| ReferenceLink {
                    target: link.target.clone(),
                    anchor: link.anchor.clone(),
                })
                .collect();
            reference_pages.push(ReferencePage {
                id: document.id.as_str().to_owned(),
                final_slugs: parsed_headings,
                links: parsed_links,
            });
            parsed_documents.push(document);
        }

        let mut expected = ref_resolve(&reference_pages);
        let mut actual: Vec<WitnessTuple> = resolve(&parsed_documents)
            .witnesses
            .iter()
            .map(project_witness)
            .collect();
        expected.sort();
        actual.sort();
        prop_assert_eq!(actual, expected);
    }
}
