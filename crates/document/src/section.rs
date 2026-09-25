use crate::{Document, Node, Transclusion};

/// Selects the complete document body or one exact heading section.
#[must_use]
pub fn section_nodes(document: &Document, anchor: Option<&str>) -> Option<Vec<Node>> {
    match anchor {
        None => Some(document.body.clone()),
        Some(anchor) => select_sequence(&document.body, anchor),
    }
}

fn select_sequence(sequence: &[Node], anchor: &str) -> Option<Vec<Node>> {
    for (index, node) in sequence.iter().enumerate() {
        let Node::Heading { level, slug, .. } = node else {
            continue;
        };
        if slug != anchor {
            continue;
        }

        let end = sequence
            .iter()
            .enumerate()
            .skip(index + 1)
            .find_map(|(boundary, node)| match node {
                Node::Heading {
                    level: boundary_level,
                    ..
                } if boundary_level <= level => Some(boundary),
                _ => None,
            })
            .unwrap_or(sequence.len());
        return Some(sequence[index..end].to_vec());
    }

    for node in sequence {
        match node {
            Node::List { start, items } => {
                for (item_index, item) in items.iter().enumerate() {
                    if let Some(selected) = select_sequence(item, anchor) {
                        let selected_start =
                            start.map(|value| value.saturating_add(item_index as u64));
                        return Some(vec![Node::List {
                            start: selected_start,
                            items: vec![selected],
                        }]);
                    }
                }
            }
            Node::BlockQuote { kind, children } => {
                if let Some(selected) = select_sequence(children, anchor) {
                    return Some(vec![Node::BlockQuote {
                        kind: *kind,
                        children: selected,
                    }]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Returns transclusion descriptors in deterministic tree order.
#[must_use]
pub fn transclusions(nodes: &[Node]) -> Vec<&Transclusion> {
    let mut result = Vec::new();
    collect_transclusions(nodes, &mut result);
    result
}

fn collect_transclusions<'a>(nodes: &'a [Node], result: &mut Vec<&'a Transclusion>) {
    for node in nodes {
        match node {
            Node::Transclusion(transclusion) => result.push(transclusion),
            Node::Heading { children, .. }
            | Node::Paragraph(children)
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => collect_transclusions(children, result),
            Node::List { items, .. } => {
                for item in items {
                    collect_transclusions(item, result);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_transclusions(cell, result);
                }
                for row in rows {
                    for cell in row {
                        collect_transclusions(cell, result);
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
            | Node::Anchor { .. } => {}
        }
    }
}
