#![allow(clippy::expect_used)]

use adapter_html::HtmlRenderer;
use site::assembly::PageModel;
use site::{CalloutKind, Node, PageId, PageRenderer};

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
fn all_five_callout_kinds_render_correct_class_and_title() {
    let test_cases = [
        (CalloutKind::Note, "note", "Note"),
        (CalloutKind::Tip, "tip", "Tip"),
        (CalloutKind::Important, "important", "Important"),
        (CalloutKind::Warning, "warning", "Warning"),
        (CalloutKind::Caution, "caution", "Caution"),
    ];

    for (kind, class, title) in test_cases {
        let html = render_nodes(vec![Node::BlockQuote {
            kind: Some(kind),
            children: vec![Node::Paragraph(vec![Node::Text("Body text".to_string())])],
        }]);

        let expected = format!(
            "<blockquote class=\"callout {class}\">\n<p class=\"callout-title\">{title}</p>\n<p>Body text</p>\n</blockquote>\n"
        );
        assert!(
            html.contains(&expected),
            "expected fragment not found for {kind:?}:\nExpected:\n{expected}\nActual:\n{html}"
        );
    }
}

#[test]
fn empty_callout_body_renders_title_without_children() {
    let html = render_nodes(vec![Node::BlockQuote {
        kind: Some(CalloutKind::Tip),
        children: vec![],
    }]);

    assert!(
        html.contains(
            "<blockquote class=\"callout tip\">\n<p class=\"callout-title\">Tip</p>\n</blockquote>\n"
        ),
        "empty callout must contain title followed by closing blockquote tag:\n{html}"
    );
}

#[test]
fn plain_blockquote_has_no_class_and_no_title() {
    let html = render_nodes(vec![Node::BlockQuote {
        kind: None,
        children: vec![Node::Paragraph(vec![Node::Text("Plain text".to_string())])],
    }]);

    assert!(
        html.contains("<blockquote>\n<p>Plain text</p>\n</blockquote>\n"),
        "plain blockquote should render without callout class or title:\n{html}"
    );
    assert!(!html.contains("callout"), "{html}");
    assert!(!html.contains("callout-title"), "{html}");
}

#[test]
fn deeply_nested_callouts_render_each_level_with_own_title() {
    // 3 levels of nesting: Note -> Tip -> Caution
    let html = render_nodes(vec![Node::BlockQuote {
        kind: Some(CalloutKind::Note),
        children: vec![
            Node::Paragraph(vec![Node::Text("Level 1".to_string())]),
            Node::BlockQuote {
                kind: Some(CalloutKind::Tip),
                children: vec![
                    Node::Paragraph(vec![Node::Text("Level 2".to_string())]),
                    Node::BlockQuote {
                        kind: Some(CalloutKind::Caution),
                        children: vec![Node::Paragraph(vec![Node::Text("Level 3".to_string())])],
                    },
                ],
            },
        ],
    }]);

    let note_pos = html
        .find("<blockquote class=\"callout note\">\n<p class=\"callout-title\">Note</p>\n")
        .expect("level 1 note");
    let tip_pos = html
        .find("<blockquote class=\"callout tip\">\n<p class=\"callout-title\">Tip</p>\n")
        .expect("level 2 tip");
    let caution_pos = html
        .find("<blockquote class=\"callout caution\">\n<p class=\"callout-title\">Caution</p>\n")
        .expect("level 3 caution");

    assert!(note_pos < tip_pos && tip_pos < caution_pos);
    assert!(html.contains("</blockquote>\n</blockquote>\n</blockquote>\n"));
}

#[test]
fn stylesheet_assets_contain_all_required_selectors() {
    let assets = HtmlRenderer.assets();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].path.as_str(), "assets/style.css");

    let css = String::from_utf8(assets[0].bytes.clone()).expect("valid UTF-8 CSS");
    assert!(css.contains(".callout{"));
    assert!(css.contains(".callout-title{"));
    assert!(css.contains(".callout.note{"));
    assert!(css.contains(".callout.tip{"));
    assert!(css.contains(".callout.important{"));
    assert!(css.contains(".callout.warning{"));
    assert!(css.contains(".callout.caution{"));
}

#[test]
fn callouts_satisfy_site_renderer_contract() {
    let page = PageModel {
        id: PageId::new("callouts"),
        title: "Callouts".to_string(),
        breadcrumbs: vec![],
        toc: vec![],
        body: vec![
            Node::BlockQuote {
                kind: Some(CalloutKind::Note),
                children: vec![Node::Paragraph(vec![Node::Text("Note text".to_string())])],
            },
            Node::BlockQuote {
                kind: Some(CalloutKind::Tip),
                children: vec![],
            },
            Node::BlockQuote {
                kind: None,
                children: vec![Node::Paragraph(vec![Node::Text("Quote text".to_string())])],
            },
        ],
        links: vec![],
        backlinks: vec![],
        built_at: Some("2026-01-01T00:00:00Z".to_string()),
    };
    let violations = site::contract::page_renderer(&HtmlRenderer, &[page]);
    assert!(violations.is_empty(), "violations: {violations:?}");
}
