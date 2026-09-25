use std::collections::{BTreeMap, BTreeSet};

use super::{Assemble, AssembleContext, DefaultAssembler, LinkTarget, rebase_uri};
use crate::Node;
use library::{Corpus, PageId, RelPath, Source};
use serde_json::Value;

#[path = "../../../../xtask/tests/support/registered_package.rs"]
mod registered_package;

trait JsonValueExt {
    fn object(&self) -> &serde_json::Map<String, Value>;
    fn array(&self) -> &[Value];
    fn string(&self) -> &str;
    fn as_string_option(&self) -> Option<&str>;
}

impl JsonValueExt for Value {
    fn object(&self) -> &serde_json::Map<String, Value> {
        self.as_object()
            .unwrap_or_else(|| panic!("expected object, got {self:?}"))
    }

    fn array(&self) -> &[Value] {
        self.as_array()
            .unwrap_or_else(|| panic!("expected array, got {self:?}"))
    }

    fn string(&self) -> &str {
        self.as_str()
            .unwrap_or_else(|| panic!("expected string, got {self:?}"))
    }

    fn as_string_option(&self) -> Option<&str> {
        if self.is_null() {
            None
        } else {
            Some(
                self.as_str()
                    .unwrap_or_else(|| panic!("expected nullable string, got {self:?}")),
            )
        }
    }
}

fn exact_keys(value: &Value, expected: &[&str]) {
    let actual = value
        .object()
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "unexpected object members: {value:?}");
}

fn exact_keys_with_optional(value: &Value, required: &[&str], optional: &[&str]) {
    let actual = value
        .object()
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let allowed = required
        .iter()
        .chain(optional.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    let missing = required
        .iter()
        .copied()
        .filter(|key| !actual.iter().any(|actual| actual == key))
        .collect::<Vec<_>>();
    let unknown = actual.difference(&allowed).copied().collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "missing required object members: {missing:?}"
    );
    assert!(unknown.is_empty(), "unexpected object members: {unknown:?}");
}

fn source_corpus(case: &Value) -> Corpus {
    let sources = case
        .object()
        .get("sources")
        .unwrap()
        .array()
        .iter()
        .map(|source| {
            exact_keys(source, &["path", "text"]);
            Source::new(
                RelPath::new(source.object()["path"].string()).unwrap(),
                source.object()["text"].string().to_owned(),
            )
        })
        .collect();
    Corpus::new(sources).unwrap()
}

fn collect_page_nodes(
    nodes: &[Node],
    headings: &mut Vec<String>,
    hrefs: &mut Vec<String>,
    images: &mut Vec<String>,
    text: &mut Vec<String>,
) {
    for node in nodes {
        match node {
            Node::Heading { slug, children, .. } => {
                headings.push(slug.clone());
                collect_page_nodes(children, headings, hrefs, images, text);
            }
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children) => {
                collect_page_nodes(children, headings, hrefs, images, text);
            }
            Node::Text(value) | Node::Code(value) | Node::Html(value) => text.push(value.clone()),
            Node::CodeBlock { text: value, .. } => text.push(value.clone()),
            Node::Link { href, children } => {
                hrefs.push(href.clone());
                collect_page_nodes(children, headings, hrefs, images, text);
            }
            Node::WikiLink { children, .. } => {
                collect_page_nodes(children, headings, hrefs, images, text);
            }
            Node::Image { src, alt } => {
                images.push(src.clone());
                text.push(alt.clone());
            }
            Node::List { items, .. } => {
                for item in items {
                    collect_page_nodes(item, headings, hrefs, images, text);
                }
            }
            Node::BlockQuote { children, .. } => {
                collect_page_nodes(children, headings, hrefs, images, text);
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_page_nodes(cell, headings, hrefs, images, text);
                }
                for row in rows {
                    for cell in row {
                        collect_page_nodes(cell, headings, hrefs, images, text);
                    }
                }
            }
            Node::TaskMarker(_) | Node::Anchor { .. } => {}
            Node::Rule | Node::SoftBreak | Node::HardBreak => text.push(" ".to_owned()),
        }
    }
}

fn semantic_text(nodes: &[Node]) -> String {
    let mut headings = Vec::new();
    let mut hrefs = Vec::new();
    let mut images = Vec::new();
    let mut text = Vec::new();
    collect_page_nodes(nodes, &mut headings, &mut hrefs, &mut images, &mut text);
    text.join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn assert_page_observation(page_id: &str, page: &super::PageModel, expected: &Value) {
    exact_keys(
        expected,
        &[
            "heading_ids",
            "image_sources",
            "ordinary_hrefs",
            "text_excludes",
            "text_includes",
            "toc_anchors",
            "wiki_targets",
        ],
    );
    assert_eq!(
        page.toc
            .iter()
            .map(|entry| entry.anchor.clone())
            .collect::<Vec<_>>(),
        expected.object()["toc_anchors"]
            .array()
            .iter()
            .map(|value| value.string())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        "{page_id}: TOC"
    );

    let mut headings = Vec::new();
    let mut hrefs = Vec::new();
    let mut images = Vec::new();
    let mut text = Vec::new();
    collect_page_nodes(
        &page.body,
        &mut headings,
        &mut hrefs,
        &mut images,
        &mut text,
    );
    assert_eq!(
        headings,
        expected.object()["heading_ids"]
            .array()
            .iter()
            .map(|value| value.string())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        "{page_id}: headings"
    );
    assert_eq!(
        hrefs,
        expected.object()["ordinary_hrefs"]
            .array()
            .iter()
            .map(|value| value.string())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        "{page_id}: hrefs"
    );
    assert_eq!(
        images,
        expected.object()["image_sources"]
            .array()
            .iter()
            .map(|value| value.string())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        "{page_id}: images"
    );

    let wiki_targets = expected.object()["wiki_targets"].array();
    assert_eq!(
        page.links.len(),
        wiki_targets.len(),
        "{page_id}: complete links vector"
    );
    for (actual, wanted) in page.links.iter().zip(wiki_targets) {
        exact_keys(wanted, &["anchor", "page"]);
        match actual {
            LinkTarget::Page { id, anchor } => {
                assert_eq!(id.as_str(), wanted.object()["page"].string());
                assert_eq!(
                    anchor.as_deref(),
                    wanted.object()["anchor"].as_string_option()
                );
            }
            LinkTarget::Unresolved => panic!("{page_id}: unexpected unresolved link"),
        }
    }

    let text = semantic_text(&page.body);
    for value in expected.object()["text_includes"].array() {
        assert!(
            text.contains(value.string()),
            "{page_id}: missing text {:?}",
            value.string()
        );
    }
    for value in expected.object()["text_excludes"].array() {
        assert!(
            !text.contains(value.string()),
            "{page_id}: unexpected text {:?}",
            value.string()
        );
    }
}

#[test]
fn every_registered_uri_case_matches_the_contract() {
    let package = "uris";
    let cases = registered_package::load(package);
    for case in &cases {
        exact_keys(
            case,
            &[
                "anchors",
                "destination",
                "expected",
                "host",
                "id",
                "kind",
                "origin",
                "reference_kind",
            ],
        );
        assert_eq!(case.object()["kind"].string(), "uri");
        let anchors = case.object()["anchors"]
            .object()
            .iter()
            .map(|(key, value)| (key.clone(), value.string().to_owned()))
            .collect::<BTreeMap<_, _>>();
        let actual = rebase_uri(
            &PageId::new(case.object()["host"].string()),
            &PageId::new(case.object()["origin"].string()),
            case.object()["destination"].string(),
            case.object()["reference_kind"].string() == "image",
            &anchors,
        );
        assert_eq!(
            actual,
            case.object()["expected"].string(),
            "{}",
            case.object()["id"].string()
        );
    }
    assert_eq!(cases.len(), 153);
}

#[test]
fn every_registered_site_page_model_observation_is_exact() {
    let package = "sites";
    let cases = registered_package::load(package);
    assert_eq!(cases.len(), 12);
    for case in &cases {
        exact_keys_with_optional(
            case,
            &[
                "expected_check",
                "expected_documents",
                "expected_exit",
                "expected_pages",
                "id",
                "kind",
                "sources",
            ],
            &["selector_queries", "expected_html_contains"],
        );
        assert_eq!(case.object()["kind"].string(), "site");
        let corpus = source_corpus(case);
        let (documents, graph) = crate::analyse(&corpus);
        let reversed_documents = documents.iter().rev().cloned().collect::<Vec<_>>();
        assert_eq!(
            graph::resolve(&reversed_documents),
            graph,
            "document permutation"
        );
        let titles = documents
            .iter()
            .map(|document| (document.id.clone(), document.title.clone()))
            .collect::<BTreeMap<_, _>>();
        let lookup = |id: &PageId| titles.get(id).cloned();
        let context = AssembleContext::default();
        let models = documents
            .iter()
            .map(|document| {
                DefaultAssembler.assemble(document, &documents, &graph, &lookup, &context)
            })
            .collect::<Vec<_>>();

        let expected_pages = case.object()["expected_pages"].object();
        assert!(!expected_pages.is_empty());
        let mut observed = BTreeSet::new();
        for (id, expected) in expected_pages {
            let page_id = PageId::new(id);
            let page = models
                .iter()
                .find(|model| model.id == page_id)
                .unwrap_or_else(|| {
                    panic!("{}: expected page is absent", case.object()["id"].string())
                });
            assert!(observed.insert(page_id.clone()));
            assert_page_observation(id, page, expected);
        }
        assert_eq!(observed.len(), expected_pages.len());
    }
}
