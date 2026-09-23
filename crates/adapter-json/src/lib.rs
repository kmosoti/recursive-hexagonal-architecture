//! Deterministic JSON rendering for assembled rhawiki pages.
//!
//! A [`JsonRenderer`] is a fresh snapshot of one corpus. Construct another
//! renderer whenever that corpus changes, and pass the same source-derived
//! page models to the renderer and build pipeline. The renderer assumes the
//! supplied [`PageId`] values come from valid source-relative paths, matching
//! the domain accepted by the HTML renderer.

use serde_json::{Value, json};
use site::assembly::{LinkTarget, PageModel};
use site::{Align, CalloutKind, Node, PageId, PageRenderer, RelPath, Rendered};

/// Construction failures for [`JsonRenderer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonRendererError {
    /// More than one page has the same source-derived identity.
    DuplicatePageId { id: String },
}

/// A JSON page renderer with an immutable, precomputed search-index asset.
#[derive(Debug, Clone)]
pub struct JsonRenderer {
    search_index: Rendered,
}

impl JsonRenderer {
    /// Builds a renderer from a corpus snapshot without mutating the input.
    ///
    /// The search-index entries are ordered by page ID and its bytes are
    /// computed once. A new renderer is required after any corpus change.
    ///
    /// # Errors
    ///
    /// Returns [`JsonRendererError::DuplicatePageId`] when the corpus
    /// contains duplicate page IDs.
    pub fn new(pages: &[PageModel]) -> Result<Self, JsonRendererError> {
        let mut ordered: Vec<&PageModel> = pages.iter().collect();
        ordered.sort_by(|left, right| left.id.cmp(&right.id));

        for pair in ordered.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(JsonRendererError::DuplicatePageId {
                    id: pair[0].id.to_string(),
                });
            }
        }

        let entries: Vec<Value> = ordered.iter().map(|page| search_entry(page)).collect();
        let index = json!({
            "schema_version": 1,
            "kind": "rhawiki_search_index",
            "entries": entries,
        });

        Ok(Self {
            search_index: Rendered {
                path: asset_path(),
                bytes: pretty_json(&index),
            },
        })
    }
}

impl PageRenderer for JsonRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        let output = json!({
            "schema_version": 1,
            "kind": "rhawiki_page",
            "id": page.id.as_str(),
            "title": page.title,
            "toc": page
                .toc
                .iter()
                .map(|entry| {
                    json!({
                        "level": entry.level,
                        "text": entry.text,
                        "anchor": entry.anchor,
                    })
                })
                .collect::<Vec<_>>(),
            "breadcrumbs": page.breadcrumbs,
            "backlinks": page
                .backlinks
                .iter()
                .map(|(id, title)| json!({"id": id.as_str(), "title": title}))
                .collect::<Vec<_>>(),
            "links": page.links.iter().map(link_value).collect::<Vec<_>>(),
            "body": nodes_value(&page.body),
            "built_at": page.built_at,
        });

        Rendered {
            path: page_path(&page.id),
            bytes: pretty_json(&output),
        }
    }

    fn assets(&self) -> Vec<Rendered> {
        vec![self.search_index.clone()]
    }
}

fn page_path(id: &PageId) -> RelPath {
    let path = format!("{}.json", id.as_str());
    #[allow(clippy::expect_used)]
    RelPath::new(&path).expect("source-derived page IDs produce relative JSON paths")
}

fn asset_path() -> RelPath {
    #[allow(clippy::expect_used)]
    RelPath::new("assets/search-index.json").expect("static asset path")
}

fn pretty_json(value: &Value) -> Vec<u8> {
    #[allow(clippy::expect_used)]
    let mut bytes =
        serde_json::to_vec_pretty(value).expect("serde_json::Value serialization cannot fail");
    bytes.push(b'\n');
    bytes
}

fn link_value(link: &LinkTarget) -> Value {
    match link {
        LinkTarget::Page { id, anchor } => json!({
            "status": "resolved",
            "page": id.as_str(),
            "anchor": anchor,
        }),
        LinkTarget::Unresolved => json!({
            "status": "unresolved",
            "page": null,
            "anchor": null,
        }),
    }
}

fn nodes_value(nodes: &[Node]) -> Vec<Value> {
    nodes.iter().map(node_value).collect()
}

fn node_value(node: &Node) -> Value {
    match node {
        Node::Heading {
            level,
            slug,
            children,
        } => json!({
            "type": "heading",
            "level": level,
            "anchor": slug,
            "children": nodes_value(children),
        }),
        Node::Paragraph(children) => json!({
            "type": "paragraph",
            "children": nodes_value(children),
        }),
        Node::Emphasis(children) => json!({
            "type": "emphasis",
            "children": nodes_value(children),
        }),
        Node::Strong(children) => json!({
            "type": "strong",
            "children": nodes_value(children),
        }),
        Node::Strikethrough(children) => json!({
            "type": "strikethrough",
            "children": nodes_value(children),
        }),
        Node::Text(text) => json!({
            "type": "text",
            "text": text,
        }),
        Node::Code(text) => json!({
            "type": "code",
            "text": text,
        }),
        Node::Html(text) => json!({
            "type": "html",
            "text": text,
        }),
        Node::CodeBlock { lang, text } => json!({
            "type": "code_block",
            "language": lang,
            "text": text,
        }),
        Node::Link { href, children } => json!({
            "type": "link",
            "href": href,
            "children": nodes_value(children),
        }),
        Node::WikiLink { index, children } => json!({
            "type": "wiki_link",
            "link_index": index,
            "children": nodes_value(children),
        }),
        Node::Image { src, alt } => json!({
            "type": "image",
            "src": src,
            "alt": alt,
        }),
        Node::List { start, items } => json!({
            "type": "list",
            "start": start,
            "items": items
                .iter()
                .map(|item| nodes_value(item))
                .collect::<Vec<_>>(),
        }),
        Node::BlockQuote { kind, children } => json!({
            "type": "block_quote",
            "kind": callout_kind(*kind),
            "children": nodes_value(children),
        }),
        Node::Table { align, head, rows } => json!({
            "type": "table",
            "align": align
                .iter()
                .map(|alignment| alignment_value(*alignment))
                .collect::<Vec<_>>(),
            "head": head
                .iter()
                .map(|cell| nodes_value(cell))
                .collect::<Vec<_>>(),
            "rows": rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| nodes_value(cell))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
        }),
        Node::TaskMarker(checked) => json!({
            "type": "task_marker",
            "checked": checked,
        }),
        Node::Rule => json!({"type": "rule"}),
        Node::SoftBreak => json!({"type": "soft_break"}),
        Node::HardBreak => json!({"type": "hard_break"}),
    }
}

fn callout_kind(kind: Option<CalloutKind>) -> Option<&'static str> {
    kind.map(|kind| match kind {
        CalloutKind::Note => "note",
        CalloutKind::Tip => "tip",
        CalloutKind::Important => "important",
        CalloutKind::Warning => "warning",
        CalloutKind::Caution => "caution",
    })
}

fn alignment_value(alignment: Align) -> &'static str {
    match alignment {
        Align::None => "none",
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}

fn search_entry(page: &PageModel) -> Value {
    json!({
        "id": page.id.as_str(),
        "path": format!("{}.json", page.id.as_str()),
        "title": page.title,
        "headings": page
            .toc
            .iter()
            .map(|entry| {
                json!({
                    "level": entry.level,
                    "text": entry.text,
                    "anchor": entry.anchor,
                })
            })
            .collect::<Vec<_>>(),
        "text": search_text(&page.body),
    })
}

struct SearchText {
    value: String,
}

impl SearchText {
    fn new() -> Self {
        Self {
            value: String::new(),
        }
    }

    fn boundary(&mut self) {
        self.value.push(' ');
    }

    fn append(&mut self, text: &str) {
        self.value.push_str(text);
    }

    fn nodes(&mut self, nodes: &[Node]) {
        for node in nodes {
            self.node(node);
        }
    }

    fn bounded_nodes(&mut self, nodes: &[Node]) {
        self.boundary();
        self.nodes(nodes);
        self.boundary();
    }

    fn node(&mut self, node: &Node) {
        match node {
            Node::Heading { children, .. } | Node::Paragraph(children) => {
                self.bounded_nodes(children);
            }
            Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. } => self.nodes(children),
            Node::Text(text) | Node::Code(text) | Node::Html(text) => self.append(text),
            Node::CodeBlock { text, .. } => {
                self.boundary();
                self.append(text);
                self.boundary();
            }
            Node::Image { alt, .. } => self.append(alt),
            Node::List { items, .. } => {
                for item in items {
                    self.bounded_nodes(item);
                }
            }
            Node::BlockQuote { children, .. } => self.bounded_nodes(children),
            Node::Table { head, rows, .. } => {
                for cell in head {
                    self.bounded_nodes(cell);
                }
                for row in rows {
                    for cell in row {
                        self.bounded_nodes(cell);
                    }
                }
            }
            Node::TaskMarker(_) => {}
            Node::Rule | Node::SoftBreak | Node::HardBreak => self.boundary(),
        }
    }
}

fn search_text(body: &[Node]) -> String {
    let mut text = SearchText::new();
    text.nodes(body);
    text.value.split_whitespace().collect::<Vec<_>>().join(" ")
}
