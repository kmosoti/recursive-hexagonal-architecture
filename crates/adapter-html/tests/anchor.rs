use adapter_html::HtmlRenderer;
use site::assembly::PageModel;
use site::{Node, PageId, PageRenderer};

fn render_nodes(nodes: Vec<Node>) -> String {
    let page = PageModel {
        id: PageId::new("test"),
        title: "Test".to_string(),
        breadcrumbs: vec![],
        toc: vec![],
        body: nodes,
        links: vec![],
        backlinks: vec![],
        built_at: Some("2026-01-01T00:00:00Z".to_string()),
    };
    let rendered = HtmlRenderer.render(&page);
    String::from_utf8(rendered.bytes).expect("valid UTF-8")
}

#[test]
fn anchor_renders_empty_a_tag_with_id() {
    let html = render_nodes(vec![Node::Paragraph(vec![
        Node::Anchor {
            id: "ref-r1".to_string(),
        },
        Node::Text("[R1] Author. Title.".to_string()),
    ])]);
    assert!(
        html.contains("<p><a id=\"ref-r1\"></a>[R1] Author. Title.</p>"),
        "{html}"
    );
}

#[test]
fn anchor_id_escapes_special_characters() {
    let html = render_nodes(vec![Node::Anchor {
        id: "test&<\"'>ref".to_string(),
    }]);
    assert!(
        html.contains("<a id=\"test&amp;&lt;&quot;&#39;&gt;ref\"></a>"),
        "{html}"
    );
}

#[test]
fn anchor_empty_id_renders_empty_attribute() {
    let html = render_nodes(vec![Node::Anchor { id: String::new() }]);
    assert!(html.contains("<a id=\"\"></a>"), "{html}");
}

#[test]
fn multiple_adjacent_anchors_render_without_whitespace() {
    let html = render_nodes(vec![Node::Paragraph(vec![
        Node::Anchor {
            id: "ref-r1".to_string(),
        },
        Node::Anchor {
            id: "ref-r2".to_string(),
        },
        Node::Text("text".to_string()),
    ])]);
    assert!(
        html.contains("<p><a id=\"ref-r1\"></a><a id=\"ref-r2\"></a>text</p>"),
        "{html}"
    );
}

#[test]
fn anchor_in_block_quote_and_list() {
    let html = render_nodes(vec![
        Node::BlockQuote {
            kind: None,
            children: vec![Node::Paragraph(vec![
                Node::Anchor {
                    id: "ref-quote".to_string(),
                },
                Node::Text("Quoted reference".to_string()),
            ])],
        },
        Node::List {
            start: None,
            items: vec![vec![
                Node::Anchor {
                    id: "ref-item".to_string(),
                },
                Node::Text("Item reference".to_string()),
            ]],
        },
    ]);
    assert!(
        html.contains(
            "<blockquote>\n<p><a id=\"ref-quote\"></a>Quoted reference</p>\n</blockquote>"
        ),
        "{html}"
    );
    assert!(
        html.contains("<li><a id=\"ref-item\"></a>Item reference</li>"),
        "{html}"
    );
}

#[test]
fn anchor_satisfies_site_renderer_contract() {
    let page = PageModel {
        id: PageId::new("refs"),
        title: "Refs".to_string(),
        breadcrumbs: vec![],
        toc: vec![],
        body: vec![Node::Paragraph(vec![
            Node::Anchor {
                id: "ref-r1".to_string(),
            },
            Node::Text("[R1] Author. Title.".to_string()),
        ])],
        links: vec![],
        backlinks: vec![],
        built_at: Some("2026-01-01T00:00:00Z".to_string()),
    };
    let violations = site::contract::page_renderer(&HtmlRenderer, &[page]);
    assert!(violations.is_empty(), "violations: {violations:?}");
}
