#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../../xtask/tests/support/registered_package.rs"]
mod registered_package;

use std::collections::BTreeSet;

use document::{
    Align, CalloutKind, Document, Heading, Node, Transclusion, parse, section_nodes, transclusions,
};
use library::{Digest, PageId, RelPath, Source};
use serde_json::{Map, Value, json};

fn object<'a>(value: &'a Value, keys: &[&str], context: &str) -> &'a Map<String, Value> {
    let object = value.as_object().expect(context);
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = keys.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context}");
    object
}

fn string(object: &Map<String, Value>, key: &str) -> String {
    object[key].as_str().expect(key).to_owned()
}

fn number<T>(object: &Map<String, Value>, key: &str) -> T
where
    T: TryFrom<u64>,
{
    object[key]
        .as_u64()
        .expect(key)
        .try_into()
        .ok()
        .expect("checked numeric conversion")
}

fn nodes(value: &Value, context: &str) -> Vec<Node> {
    value
        .as_array()
        .expect(context)
        .iter()
        .enumerate()
        .map(|(index, value)| node(value, &format!("{context}[{index}]")))
        .collect()
}

fn optional_string(value: &Value, context: &str) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        _ => panic!("{context} must be null or a string"),
    }
}

fn callout(value: &Value, context: &str) -> Option<CalloutKind> {
    match optional_string(value, context).as_deref() {
        None => None,
        Some("Note") => Some(CalloutKind::Note),
        Some("Tip") => Some(CalloutKind::Tip),
        Some("Important") => Some(CalloutKind::Important),
        Some("Warning") => Some(CalloutKind::Warning),
        Some("Caution") => Some(CalloutKind::Caution),
        Some(other) => panic!("{context}: unknown callout {other}"),
    }
}

fn align(value: &Value, context: &str) -> Align {
    match value.as_str().expect(context) {
        "None" => Align::None,
        "Left" => Align::Left,
        "Center" => Align::Center,
        "Right" => Align::Right,
        other => panic!("{context}: unknown alignment {other}"),
    }
}

fn table_cells(value: &Value, context: &str) -> Vec<Vec<Node>> {
    value
        .as_array()
        .expect(context)
        .iter()
        .enumerate()
        .map(|(index, value)| nodes(value, &format!("{context}[{index}]")))
        .collect()
}

fn table_rows(value: &Value, context: &str) -> Vec<Vec<Vec<Node>>> {
    value
        .as_array()
        .expect(context)
        .iter()
        .enumerate()
        .map(|(index, value)| table_cells(value, &format!("{context}[{index}]")))
        .collect()
}

fn node(value: &Value, context: &str) -> Node {
    let kind = value["kind"].as_str().expect(context);
    let object = match kind {
        "Heading" => object(value, &["kind", "level", "slug", "children"], context),
        "Paragraph" | "Emphasis" | "Strong" | "Strikethrough" => {
            object(value, &["kind", "children"], context)
        }
        "Text" | "Code" | "Html" => object(value, &["kind", "text"], context),
        "CodeBlock" => object(value, &["kind", "lang", "text"], context),
        "Link" => object(value, &["kind", "href", "children"], context),
        "WikiLink" => object(value, &["kind", "index", "children"], context),
        "Image" => object(value, &["kind", "src", "alt"], context),
        "List" => object(value, &["kind", "start", "items"], context),
        "BlockQuote" => object(value, &["kind", "callout", "children"], context),
        "Table" => object(value, &["kind", "align", "head", "rows"], context),
        "Rule" | "SoftBreak" | "HardBreak" => object(value, &["kind"], context),
        "TaskMarker" => object(value, &["kind", "checked"], context),
        "Transclusion" => object(
            value,
            &["kind", "id", "target", "anchor", "display", "line"],
            context,
        ),
        other => panic!("{context}: unknown node kind {other}"),
    };

    match kind {
        "Heading" => Node::Heading {
            level: number(object, "level"),
            slug: string(object, "slug"),
            children: nodes(&object["children"], context),
        },
        "Paragraph" => Node::Paragraph(nodes(&object["children"], context)),
        "Text" => Node::Text(string(object, "text")),
        "Code" => Node::Code(string(object, "text")),
        "CodeBlock" => Node::CodeBlock {
            lang: optional_string(&object["lang"], context),
            text: string(object, "text"),
        },
        "Emphasis" => Node::Emphasis(nodes(&object["children"], context)),
        "Strong" => Node::Strong(nodes(&object["children"], context)),
        "Strikethrough" => Node::Strikethrough(nodes(&object["children"], context)),
        "Link" => Node::Link {
            href: string(object, "href"),
            children: nodes(&object["children"], context),
        },
        "WikiLink" => Node::WikiLink {
            index: number(object, "index"),
            children: nodes(&object["children"], context),
        },
        "Image" => Node::Image {
            src: string(object, "src"),
            alt: string(object, "alt"),
        },
        "List" => Node::List {
            start: match &object["start"] {
                Value::Null => None,
                value => Some(value.as_u64().expect("list start")),
            },
            items: object["items"]
                .as_array()
                .expect("list items")
                .iter()
                .enumerate()
                .map(|(index, value)| nodes(value, &format!("{context}.items[{index}]")))
                .collect(),
        },
        "BlockQuote" => Node::BlockQuote {
            kind: callout(&object["callout"], context),
            children: nodes(&object["children"], context),
        },
        "Table" => Node::Table {
            align: object["align"]
                .as_array()
                .expect("table alignment")
                .iter()
                .enumerate()
                .map(|(index, value)| align(value, &format!("{context}.align[{index}]")))
                .collect(),
            head: table_cells(&object["head"], context),
            rows: table_rows(&object["rows"], context),
        },
        "Rule" => Node::Rule,
        "SoftBreak" => Node::SoftBreak,
        "HardBreak" => Node::HardBreak,
        "Html" => Node::Html(string(object, "text")),
        "TaskMarker" => Node::TaskMarker(object["checked"].as_bool().expect("task marker")),
        "Transclusion" => Node::Transclusion(Transclusion {
            id: number(object, "id"),
            target: string(object, "target"),
            anchor: optional_string(&object["anchor"], context),
            display: string(object, "display"),
            line: number(object, "line"),
        }),
        _ => unreachable!(),
    }
}

fn plain(nodes: &[Node]) -> String {
    let mut output = String::new();
    for node in nodes {
        match node {
            Node::Text(text) | Node::Code(text) | Node::Html(text) => output.push_str(text),
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => output.push_str(&plain(children)),
            Node::List { items, .. } => {
                for item in items {
                    output.push_str(&plain(item));
                }
            }
            Node::Table { head, rows, .. } => {
                output.push_str(&plain(&head.iter().flatten().cloned().collect::<Vec<_>>()));
                for row in rows {
                    output.push_str(&plain(&row.iter().flatten().cloned().collect::<Vec<_>>()));
                }
            }
            Node::CodeBlock { text, .. } => output.push_str(text),
            Node::Image { alt, .. } => output.push_str(alt),
            Node::TaskMarker(_) | Node::Rule | Node::SoftBreak | Node::HardBreak => {}
            Node::Transclusion(Transclusion { display, .. }) => output.push_str(display),
        }
    }
    output
}

fn headings(nodes: &[Node], output: &mut Vec<Heading>) {
    for node in nodes {
        match node {
            Node::Heading {
                level,
                slug,
                children,
            } => {
                output.push(Heading {
                    level: *level,
                    text: plain(children),
                    slug: slug.clone(),
                    base_slug: slug.clone(),
                    line: 0,
                });
                headings(children, output);
            }
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => headings(children, output),
            Node::List { items, .. } => {
                for item in items {
                    headings(item, output);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head.iter().flatten() {
                    headings(std::slice::from_ref(cell), output);
                }
                for row in rows {
                    for cell in row.iter().flatten() {
                        headings(std::slice::from_ref(cell), output);
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

fn fixture_document(body: Vec<Node>) -> Document {
    let mut document_headings = Vec::new();
    headings(&body, &mut document_headings);
    Document {
        id: PageId::new("fixture"),
        source_digest: Digest::of(b"registered-section-fixture"),
        title: "fixture".to_owned(),
        headings: document_headings,
        links: Vec::new(),
        body,
        diagnostics: Vec::new(),
        section_refs: Vec::new(),
    }
}

fn descriptor_json(transclusion: &Transclusion) -> Value {
    json!({
        "kind": "Transclusion",
        "id": transclusion.id,
        "target": transclusion.target,
        "anchor": transclusion.anchor,
        "display": transclusion.display,
        "line": transclusion.line,
    })
}

#[test]
fn all_registered_section_cases_grade_exact_selection_and_descriptors() {
    let cases = registered_package::load("sections");
    assert_eq!(cases.len(), 80);

    for case in cases {
        let id = case["id"].as_str().expect("section id");
        let body = nodes(&case["body"], id);
        let document = fixture_document(body);
        let before = document.clone();
        let selected = section_nodes(&document, case["anchor"].as_str());

        match case["expected"].as_array() {
            Some(expected) => {
                let actual = selected.as_ref().expect("expected selected section");
                let expected = expected
                    .iter()
                    .enumerate()
                    .map(|(index, value)| node(value, &format!("{id}.expected[{index}]")))
                    .collect::<Vec<_>>();
                assert_eq!(actual, &expected, "{id}: selected IR");
                let actual_descriptors = transclusions(actual)
                    .into_iter()
                    .map(descriptor_json)
                    .collect::<Vec<_>>();
                assert_eq!(
                    Value::Array(actual_descriptors),
                    case["expected_transclusions"],
                    "{id}: descriptors"
                );
            }
            None => {
                assert!(selected.is_none(), "{id}: missing anchor must return None");
                assert_eq!(
                    case["expected_transclusions"],
                    json!([]),
                    "{id}: absent descriptors"
                );
            }
        }
        assert_eq!(document, before, "{id}: selector mutated input");
    }
}

fn source_document(path: &str, text: &str) -> Document {
    let rel_path = RelPath::new(&format!("{path}.md")).expect("fixture path");
    parse(&Source::new(rel_path, text.to_owned()))
}

fn visit_images(nodes: &[Node], output: &mut Vec<Value>) {
    for node in nodes {
        match node {
            Node::Image { src, alt } => output.push(json!({"src": src, "alt": alt})),
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => visit_images(children, output),
            Node::List { items, .. } => {
                for item in items {
                    visit_images(item, output);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head.iter().flatten() {
                    visit_images(std::slice::from_ref(cell), output);
                }
                for row in rows {
                    for cell in row.iter().flatten() {
                        visit_images(std::slice::from_ref(cell), output);
                    }
                }
            }
            Node::Text(_)
            | Node::Code(_)
            | Node::CodeBlock { .. }
            | Node::Rule
            | Node::SoftBreak
            | Node::HardBreak
            | Node::Html(_)
            | Node::TaskMarker(_)
            | Node::Transclusion(_) => {}
        }
    }
}

fn document_projection(document: &Document) -> Value {
    let transclusions = transclusions(&document.body)
        .into_iter()
        .map(|descriptor| {
            json!({
                "id": descriptor.id,
                "target": descriptor.target,
                "anchor": descriptor.anchor,
                "display": descriptor.display,
                "line": descriptor.line,
            })
        })
        .collect::<Vec<_>>();
    let navigation_links = document
        .links
        .iter()
        .map(|link| {
            json!({
                "target": link.target,
                "anchor": link.anchor,
                "alias": link.alias,
                "line": link.line,
            })
        })
        .collect::<Vec<_>>();
    let mut images = Vec::new();
    visit_images(&document.body, &mut images);
    json!({
        "transclusions": transclusions,
        "navigation_links": navigation_links,
        "images": images,
        "heading_slugs": document.headings.iter().map(|heading| heading.slug.clone()).collect::<Vec<_>>(),
    })
}

fn heading_slugs(nodes: &[Node], output: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Heading { slug, children, .. } => {
                output.push(slug.clone());
                heading_slugs(children, output);
            }
            Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => heading_slugs(children, output),
            Node::List { items, .. } => {
                for item in items {
                    heading_slugs(item, output);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head.iter().flatten() {
                    heading_slugs(std::slice::from_ref(cell), output);
                }
                for row in rows {
                    for cell in row.iter().flatten() {
                        heading_slugs(std::slice::from_ref(cell), output);
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
fn every_source_in_every_site_case_has_an_exact_document_projection() {
    let cases = registered_package::load("sites");
    assert_eq!(cases.len(), 12);

    for case in cases {
        let id = case["id"].as_str().expect("site id");
        let sources = case["sources"].as_array().expect("sources");
        let expected_documents = case["expected_documents"]
            .as_object()
            .expect("expected documents");
        let mut actual_paths = BTreeSet::new();

        for source in sources {
            let source = source.as_object().expect("source");
            let path = string(source, "path");
            let text = string(source, "text");
            let document = source_document(&path, &text);
            assert!(actual_paths.insert(path.clone()));
            assert_eq!(
                document_projection(&document),
                expected_documents[&path],
                "{id}/{path}"
            );
        }

        let expected_paths = expected_documents.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(actual_paths, expected_paths, "{id}: source closure");
    }
}

#[test]
fn optional_selector_queries_grade_selected_and_excluded_headings_and_wrapper_shape() {
    let cases = registered_package::load("sites");
    let mut query_count = 0;
    for case in cases
        .iter()
        .filter(|case| case.get("selector_queries").is_some())
    {
        let id = case["id"].as_str().expect("site id");
        let sources = case["sources"].as_array().expect("sources");

        for query in case["selector_queries"]
            .as_array()
            .expect("selector queries")
        {
            query_count += 1;
            let query = object(
                query,
                &[
                    "source",
                    "anchor",
                    "selected_heading_slugs",
                    "excluded_heading_slugs",
                    "wrapper",
                    "ordered_start",
                ],
                "selector query",
            );
            let source = sources
                .iter()
                .find(|source| source["path"] == query["source"])
                .unwrap_or_else(|| panic!("{id}: selector source not found"));
            let source = source.as_object().expect("source");
            let document = source_document(
                source["path"].as_str().expect("source path"),
                source["text"].as_str().expect("source text"),
            );

            let selected = section_nodes(&document, query["anchor"].as_str()).expect("selection");
            let mut selected_slugs = Vec::new();
            heading_slugs(&selected, &mut selected_slugs);
            assert_eq!(json!(selected_slugs), query["selected_heading_slugs"]);

            let selected_set = selected_slugs.iter().collect::<BTreeSet<_>>();
            let excluded = document
                .headings
                .iter()
                .map(|heading| heading.slug.clone())
                .filter(|slug| !selected_set.contains(slug))
                .collect::<Vec<_>>();
            assert_eq!(json!(excluded), query["excluded_heading_slugs"]);

            assert_eq!(
                query["wrapper"], "ordered-list-item-2-block-quote",
                "wrapper taxonomy"
            );
            let Node::List { start, items } = &selected[0] else {
                panic!("selector wrapper must retain the ordered list");
            };
            assert_eq!(
                *start,
                Some(query["ordered_start"].as_u64().expect("ordered start"))
            );
            assert_eq!(items.len(), 1);
            assert!(matches!(items[0].as_slice(), [Node::BlockQuote { .. }]));
        }
    }
    assert_eq!(query_count, 1, "registered selector query count");
}

#[test]
fn registered_case_shape_validator_rejects_nested_key_drift() {
    let cases = registered_package::load("sections");
    let id = cases[0]["id"].as_str().expect("section id");
    let verified = Value::Array(cases.clone());
    registered_package::validate_cases("sections", &verified).expect("verified cases");

    let mut missing = verified.clone();
    let text = missing[0]["body"][0]["children"][0]
        .as_object_mut()
        .expect("nested text");
    text.remove("text").expect("known nested key");
    let error = registered_package::validate_cases("sections", &missing).expect_err("missing key");
    assert_eq!(
        error,
        format!("{id}: unexpected keys: actual={{\"kind\"}}, expected={{\"kind\", \"text\"}}")
    );

    let mut unknown = verified;
    let text = unknown[0]["body"][0]["children"][0]
        .as_object_mut()
        .expect("nested text");
    text.insert("unexpected".to_owned(), Value::Null);
    let error = registered_package::validate_cases("sections", &unknown).expect_err("unknown key");
    assert_eq!(
        error,
        format!(
            "{id}: unexpected keys: actual={{\"kind\", \"text\", \"unexpected\"}}, \
             expected={{\"kind\", \"text\"}}"
        )
    );
}
