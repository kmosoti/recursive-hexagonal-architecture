//! Milestone 2 integration tests for citation assembly in `site`:
//! - `PageNode::Anchor` assembly in host pages.
//! - Dropping of `Node::Anchor` in transcluded content.
//! - Rebasing of citation links in transcluded content.
//! - Preservation of host page anchors when transcluding external content.
//! - Transclusion dropping in nested blocks (lists and blockquotes).
//! - Nested transclusion chains dropping anchors at every level.
//! - Pages without citations or reference entries assembling unaltered.
//! - Subdirectory transclusion citation link rebasing.

use document::Document;
use library::load;
use library::testing::MemorySources;
use site::assembly::{Assemble, AssembleContext, DefaultAssembler, PageModel};
use site::{Node, analyse};

fn assemble_corpus(sources: &[(&str, &str)]) -> (Vec<Document>, Vec<PageModel>) {
    let (corpus, _) = load(&MemorySources::new(sources)).expect("valid corpus");
    let (docs, graph) = analyse(&corpus);
    let no_titles = |_: &library::PageId| None;
    let ctx = AssembleContext::default();
    let pages = docs
        .iter()
        .map(|d| DefaultAssembler.assemble(d, &docs, &graph, &no_titles, &ctx))
        .collect();
    (docs, pages)
}

fn collect_anchors(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Anchor { id } => out.push(id.clone()),
            Node::Paragraph(children)
            | Node::Heading { children, .. }
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => collect_anchors(children, out),
            Node::List { items, .. } => {
                for item in items {
                    collect_anchors(item, out);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_anchors(cell, out);
                }
                for row in rows {
                    for cell in row {
                        collect_anchors(cell, out);
                    }
                }
            }
            Node::Text(_)
            | Node::Code(_)
            | Node::CodeBlock { .. }
            | Node::Html(_)
            | Node::Image { .. }
            | Node::Rule
            | Node::SoftBreak
            | Node::HardBreak
            | Node::TaskMarker(_) => {}
        }
    }
}

fn page_anchors(page: &PageModel) -> Vec<String> {
    let mut anchors = Vec::new();
    collect_anchors(&page.body, &mut anchors);
    anchors
}

fn collect_links(nodes: &[Node], out: &mut Vec<(String, String)>) {
    for node in nodes {
        match node {
            Node::Link { href, children } => {
                let mut text = String::new();
                for child in children {
                    if let Node::Text(t) = child {
                        text.push_str(t);
                    }
                }
                out.push((href.clone(), text));
                collect_links(children, out);
            }
            Node::Paragraph(children)
            | Node::Heading { children, .. }
            | Node::Emphasis(children)
            | Node::Strong(children)
            | Node::Strikethrough(children)
            | Node::WikiLink { children, .. }
            | Node::BlockQuote { children, .. } => collect_links(children, out),
            Node::List { items, .. } => {
                for item in items {
                    collect_links(item, out);
                }
            }
            Node::Table { head, rows, .. } => {
                for cell in head {
                    collect_links(cell, out);
                }
                for row in rows {
                    for cell in row {
                        collect_links(cell, out);
                    }
                }
            }
            Node::Anchor { .. }
            | Node::Text(_)
            | Node::Code(_)
            | Node::CodeBlock { .. }
            | Node::Html(_)
            | Node::Image { .. }
            | Node::Rule
            | Node::SoftBreak
            | Node::HardBreak
            | Node::TaskMarker(_) => {}
        }
    }
}

#[test]
fn host_page_assembles_anchor_nodes_for_reference_entries() {
    let (_, pages) = assemble_corpus(&[(
        "refs.md",
        "# References\n\n[R1] Author One. (2020).\n\n[R2] Author Two. (2021).\n",
    )]);

    let refs = &pages[0];
    assert_eq!(refs.id.as_str(), "refs");
    assert_eq!(page_anchors(refs), vec!["ref-r1", "ref-r2"]);

    // Verify anchor node structure inside paragraph
    match &refs.body[1] {
        Node::Paragraph(children) => {
            assert!(matches!(&children[0], Node::Anchor { id } if id == "ref-r1"));
            assert!(matches!(&children[1], Node::Text(t) if t == "[R1]"));
            assert!(matches!(&children[2], Node::Text(t) if t == " Author One. (2020)."));
        }
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn transclusion_drops_anchor_nodes_from_imported_content() {
    let (_, pages) = assemble_corpus(&[
        (
            "refs.md",
            "# References Page\n\n## Bibliography\n\n[R1] Author One. (2020).\n\n[R2] Author Two. (2021).\n",
        ),
        (
            "host.md",
            "# Host Page\n\nbefore\n\n![[refs#bibliography]]\n\nafter\n",
        ),
    ]);

    let refs = pages.iter().find(|p| p.id.as_str() == "refs").unwrap();
    let host = pages.iter().find(|p| p.id.as_str() == "host").unwrap();

    // Source page retains anchors
    assert_eq!(page_anchors(refs), vec!["ref-r1", "ref-r2"]);

    // Host page drops all transcluded anchors
    assert!(
        page_anchors(host).is_empty(),
        "host page must not contain any anchors from transcluded content: {:?}",
        page_anchors(host)
    );

    // Host page still contains the entry text
    let mut host_text = Vec::new();
    fn extract_text(nodes: &[Node], out: &mut Vec<String>) {
        for n in nodes {
            match n {
                Node::Text(t) => out.push(t.clone()),
                Node::Paragraph(c) | Node::Heading { children: c, .. } => extract_text(c, out),
                _ => {}
            }
        }
    }
    extract_text(&host.body, &mut host_text);
    let full_text = host_text.join(" ");
    assert!(full_text.contains("Author One"));
    assert!(full_text.contains("Author Two"));
}

#[test]
fn host_page_preserves_own_anchors_while_dropping_transcluded_anchors() {
    let (_, pages) = assemble_corpus(&[
        (
            "origin.md",
            "# Origin\n\n## Entries\n\n[R1] Origin entry.\n",
        ),
        (
            "host.md",
            "# Host\n\n[R10] Host entry.\n\n![[origin#entries]]\n",
        ),
    ]);

    let host = pages.iter().find(|p| p.id.as_str() == "host").unwrap();
    // Host has its own ref-r10 anchor, but NOT the transcluded ref-r1 anchor!
    assert_eq!(page_anchors(host), vec!["ref-r10"]);
}

#[test]
fn transcluded_citation_links_rebase_to_origin_page() {
    let (_, pages) = assemble_corpus(&[
        (
            "refs.md",
            "# References\n\n[R1] Author.\n\n## Discussion\n\nSee [R1].\n",
        ),
        ("host.md", "# Host\n\n![[refs#discussion]]\n"),
    ]);

    let refs = pages.iter().find(|p| p.id.as_str() == "refs").unwrap();
    let host = pages.iter().find(|p| p.id.as_str() == "host").unwrap();

    let mut refs_links = Vec::new();
    collect_links(&refs.body, &mut refs_links);
    assert_eq!(refs_links, vec![("#ref-r1".to_owned(), "[R1]".to_owned())]);

    let mut host_links = Vec::new();
    collect_links(&host.body, &mut host_links);
    // In transcluded content, #ref-r1 is rebased to refs.md#ref-r1
    assert_eq!(
        host_links,
        vec![("refs.md#ref-r1".to_owned(), "[R1]".to_owned())]
    );
}

#[test]
fn transclusion_dropping_applies_to_nested_blocks_list_and_quote() {
    let (_, pages) = assemble_corpus(&[
        (
            "nested.md",
            "# Nested\n\n## Section\n\n- [R1] In list.\n\n> [R2] In quote.\n",
        ),
        ("host.md", "# Host\n\n![[nested#section]]\n"),
    ]);

    let nested = pages.iter().find(|p| p.id.as_str() == "nested").unwrap();
    let host = pages.iter().find(|p| p.id.as_str() == "host").unwrap();

    assert_eq!(page_anchors(nested), vec!["ref-r1", "ref-r2"]);
    assert!(page_anchors(host).is_empty());
}

#[test]
fn nested_transclusion_chains_drop_anchors_at_every_level() {
    let (_, pages) = assemble_corpus(&[
        ("c.md", "# C\n\n## Sec\n\n[R1] Author.\n"),
        ("b.md", "# B\n\n## Sec\n\n![[c#sec]]\n"),
        ("a.md", "# A\n\n![[b#sec]]\n"),
    ]);

    let c = pages.iter().find(|p| p.id.as_str() == "c").unwrap();
    let b = pages.iter().find(|p| p.id.as_str() == "b").unwrap();
    let a = pages.iter().find(|p| p.id.as_str() == "a").unwrap();

    assert_eq!(page_anchors(c), vec!["ref-r1"]);
    assert!(page_anchors(b).is_empty());
    assert!(page_anchors(a).is_empty());
}

#[test]
fn pages_without_citations_or_reference_entries_assemble_unaltered() {
    let (_, pages) =
        assemble_corpus(&[("clean.md", "# Simple Title\n\nJust a normal paragraph.\n")]);

    let clean = &pages[0];
    assert!(page_anchors(clean).is_empty());
    assert_eq!(clean.body.len(), 2);
    assert!(
        matches!(&clean.body[0], Node::Heading { level: 1, slug, .. } if slug == "simple-title")
    );
    assert!(matches!(&clean.body[1], Node::Paragraph(_)));
}

#[test]
fn transcluded_citation_links_rebase_across_subdirectories() {
    let (_, pages) = assemble_corpus(&[
        (
            "bib/refs.md",
            "# References\n\n[R1] Author.\n\n## Discussion\n\nSee [R1].\n",
        ),
        ("docs/host.md", "# Host\n\n![[bib/refs#discussion]]\n"),
    ]);

    let host = pages.iter().find(|p| p.id.as_str() == "docs/host").unwrap();

    let mut host_links = Vec::new();
    collect_links(&host.body, &mut host_links);
    assert_eq!(
        host_links,
        vec![("../bib/refs.md#ref-r1".to_owned(), "[R1]".to_owned())]
    );
}
