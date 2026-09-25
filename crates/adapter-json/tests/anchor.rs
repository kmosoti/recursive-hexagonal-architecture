#[path = "common/mod.rs"]
mod common;

use adapter_json::JsonRenderer;
use serde_json::Value;
use site::assembly::PageModel;
use site::{Node, PageId, PageRenderer};

fn render_page(body: Vec<Node>) -> (Value, Value) {
    let page = PageModel {
        id: PageId::new("test"),
        title: "Test".to_string(),
        breadcrumbs: vec![],
        toc: vec![],
        body,
        links: vec![],
        backlinks: vec![],
        built_at: Some("2026-01-01T00:00:00Z".to_string()),
    };
    let renderer = JsonRenderer::new(std::slice::from_ref(&page)).expect("renderer constructor");
    let rendered = renderer.render(&page);
    let page_json: Value = serde_json::from_slice(&rendered.bytes).expect("page json");
    let assets = renderer.assets();
    assert_eq!(assets.len(), 1, "expected exactly 1 asset");
    let search_json: Value = serde_json::from_slice(&assets[0].bytes).expect("search json");
    (page_json, search_json)
}

#[test]
fn anchor_node_serialization_has_exact_two_keys() {
    let (page_json, _) = render_page(vec![Node::Paragraph(vec![
        Node::Anchor {
            id: "ref-r1".to_string(),
        },
        Node::Text("[R1] Content".to_string()),
    ])]);
    let body = page_json["body"].as_array().expect("body array");
    let para = &body[0];
    let children = para["children"].as_array().expect("children array");
    let anchor = &children[0];

    assert_eq!(anchor["type"], "anchor");
    assert_eq!(anchor["id"], "ref-r1");
    let map = anchor.as_object().expect("object");
    assert_eq!(
        map.len(),
        2,
        "anchor node must have exactly two keys: {anchor}"
    );
}

#[test]
fn anchor_node_excluded_from_search_index_text() {
    let (_, search_json) = render_page(vec![Node::Paragraph(vec![
        Node::Anchor {
            id: "ref-secret-id".to_string(),
        },
        Node::Text("[R1] Author Name".to_string()),
    ])]);
    let entries = search_json["entries"].as_array().expect("entries array");
    let test_entry = entries.iter().find(|e| e["id"] == "test").expect("entry");
    let text = test_entry["text"].as_str().expect("text");

    assert!(
        !text.contains("ref-secret-id"),
        "search text must not contain anchor id: {text}"
    );
    assert!(
        text.contains("[R1] Author Name"),
        "search text must contain visible text: {text}"
    );
}

#[test]
fn anchor_node_does_not_add_search_text_boundary() {
    let (_, search_json) = render_page(vec![Node::Paragraph(vec![
        Node::Text("word".to_string()),
        Node::Anchor {
            id: "ref-r1".to_string(),
        },
        Node::Text("join".to_string()),
    ])]);
    let entries = search_json["entries"].as_array().expect("entries array");
    let test_entry = entries.iter().find(|e| e["id"] == "test").expect("entry");
    let text = test_entry["text"].as_str().expect("text");

    assert_eq!(
        text, "wordjoin",
        "anchor should not insert whitespace boundary"
    );
}

#[test]
fn anchor_satisfies_json_renderer_contract() {
    let page = PageModel {
        id: PageId::new("test"),
        title: "Test".to_string(),
        breadcrumbs: vec![],
        toc: vec![],
        body: vec![Node::Paragraph(vec![
            Node::Anchor {
                id: "ref-r1".to_string(),
            },
            Node::Text("test".to_string()),
        ])],
        links: vec![],
        backlinks: vec![],
        built_at: Some("2026-01-01T00:00:00Z".to_string()),
    };
    let renderer = JsonRenderer::new(std::slice::from_ref(&page)).expect("renderer");
    let violations = site::contract::page_renderer(&renderer, &[page]);
    assert!(violations.is_empty(), "violations: {violations:?}");
}

#[test]
fn common_node_deserializer_parses_anchor() {
    let val = serde_json::json!({
        "kind": "Anchor",
        "id": "ref-r1",
    });
    let parsed = common::node(&val);
    assert_eq!(
        parsed,
        Node::Anchor {
            id: "ref-r1".to_string()
        }
    );
}
