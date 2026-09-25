//! `site::assembly`: a document and the site graph become a page model.
//! Pure. Uses no sibling and nothing of the glue (rha-modules.toml).

use std::collections::{BTreeMap, BTreeSet};

use document::{Align, CalloutKind, Document, Node as DocumentNode, section_nodes, slugify};
use graph::{SiteGraph, TransclusionCycle, TransclusionOccurrence, TransclusionRegion};
use library::PageId;

/// Inputs every page shares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssembleContext {
    /// The footer's time, from the glue's clock; display only.
    pub built_at: Option<String>,
}

/// The renderable page vocabulary owned by assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageNode {
    Heading {
        level: u8,
        slug: String,
        children: Vec<PageNode>,
    },
    Paragraph(Vec<PageNode>),
    Text(String),
    Code(String),
    CodeBlock {
        lang: Option<String>,
        text: String,
    },
    Emphasis(Vec<PageNode>),
    Strong(Vec<PageNode>),
    Strikethrough(Vec<PageNode>),
    Link {
        href: String,
        children: Vec<PageNode>,
    },
    WikiLink {
        index: usize,
        children: Vec<PageNode>,
    },
    Anchor {
        id: String,
    },
    Image {
        src: String,
        alt: String,
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<PageNode>>,
    },
    BlockQuote {
        kind: Option<CalloutKind>,
        children: Vec<PageNode>,
    },
    Table {
        align: Vec<Align>,
        head: Vec<Vec<PageNode>>,
        rows: Vec<Vec<Vec<PageNode>>>,
    },
    Rule,
    SoftBreak,
    HardBreak,
    Html(String),
    TaskMarker(bool),
}

/// One entry of the table of contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub level: u8,
    pub text: String,
    pub anchor: String,
}

/// Where wikilink `i` of the page goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Page { id: PageId, anchor: Option<String> },
    Unresolved,
}

/// Everything a renderer needs for one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageModel {
    pub id: PageId,
    pub title: String,
    pub toc: Vec<TocEntry>,
    /// `(label, id of the index page if it exists)` for each parent segment.
    pub breadcrumbs: Vec<String>,
    pub backlinks: Vec<(PageId, String)>,
    pub links: Vec<LinkTarget>,
    pub body: Vec<PageNode>,
    pub built_at: Option<String>,
}

/// The assembly port (provided here, required by `build`).
///
/// Assumption `site.build.assemble_deterministic`: equal inputs give equal
/// models.
pub trait Assemble {
    fn assemble(
        &self,
        doc: &Document,
        documents: &[Document],
        graph: &SiteGraph,
        titles: &dyn Fn(&PageId) -> Option<String>,
        ctx: &AssembleContext,
    ) -> PageModel;
}

/// The default assembly.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultAssembler;

impl Assemble for DefaultAssembler {
    fn assemble(
        &self,
        doc: &Document,
        documents: &[Document],
        graph: &SiteGraph,
        titles: &dyn Fn(&PageId) -> Option<String>,
        ctx: &AssembleContext,
    ) -> PageModel {
        let mut toc: Vec<TocEntry> = doc
            .headings
            .iter()
            .map(|h| TocEntry {
                level: h.level,
                text: h.text.clone(),
                anchor: h.slug.clone(),
            })
            .collect();
        let links = (0..doc.links.len())
            .map(|i| match graph.resolution(&doc.id, i) {
                Some(r) => LinkTarget::Page {
                    id: r.to.clone(),
                    anchor: r.anchor.clone(),
                },
                None => LinkTarget::Unresolved,
            })
            .collect();
        let backlinks = graph
            .backlinks
            .get(&doc.id)
            .map(|set| {
                set.iter()
                    .filter(|p| **p != doc.id)
                    .map(|p| (p.clone(), titles(p).unwrap_or_else(|| p.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let segments: Vec<&str> = doc.id.as_str().split('/').collect();
        let breadcrumbs = segments[..segments.len().saturating_sub(1)]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let mut used_slugs = BTreeSet::new();
        used_slugs.extend(doc.headings.iter().map(|heading| heading.slug.clone()));
        collect_heading_slugs(&doc.body, &mut used_slugs);
        let root_region = TransclusionRegion {
            page: doc.id.clone(),
            anchor: None,
        };
        let mut state = AssemblyState {
            host: &doc.id,
            documents,
            graph,
            links,
            used_slugs,
        };
        let mut body =
            state.convert_nodes(&doc.body, doc, &root_region, false, &BTreeMap::new(), &[]);

        if doc.front_matter.as_ref().and_then(|fm| fm.index.as_deref()) == Some("tags") {
            append_tag_index_sections(doc, documents, &mut state, &mut body, &mut toc);
        }

        PageModel {
            id: doc.id.clone(),
            title: doc.title.clone(),
            toc,
            breadcrumbs,
            backlinks,
            links: state.links,
            body,
            built_at: ctx.built_at.clone(),
        }
    }
}

struct SiteTag {
    display: String,
    pages: Vec<(PageId, String)>,
}

fn collect_site_tags(documents: &[Document]) -> Vec<SiteTag> {
    let mut sorted_docs: Vec<&Document> = documents.iter().collect();
    sorted_docs.sort_by_key(|d| &d.id);

    let mut tags_by_key: BTreeMap<String, SiteTag> = BTreeMap::new();

    for d in sorted_docs {
        if let Some(fm) = &d.front_matter {
            let mut seen_in_page = BTreeSet::new();
            for tag in &fm.tags {
                let key = tag.to_ascii_lowercase();
                if seen_in_page.insert(key.clone()) {
                    tags_by_key
                        .entry(key)
                        .and_modify(|site_tag| {
                            site_tag.pages.push((d.id.clone(), d.title.clone()));
                        })
                        .or_insert_with(|| SiteTag {
                            display: tag.clone(),
                            pages: vec![(d.id.clone(), d.title.clone())],
                        });
                }
            }
        }
    }

    let mut site_tags: Vec<SiteTag> = tags_by_key.into_values().collect();
    site_tags.sort_by(|a, b| {
        a.display
            .to_ascii_lowercase()
            .cmp(&b.display.to_ascii_lowercase())
            .then_with(|| a.display.cmp(&b.display))
            .then_with(|| a.display.as_bytes().cmp(b.display.as_bytes()))
    });
    site_tags
}

fn append_tag_index_sections(
    doc: &Document,
    documents: &[Document],
    state: &mut AssemblyState<'_>,
    body: &mut Vec<PageNode>,
    toc: &mut Vec<TocEntry>,
) {
    let site_tags = collect_site_tags(documents);
    if site_tags.is_empty() {
        return;
    }

    let page_heading_slugs: BTreeSet<&str> = doc.headings.iter().map(|h| h.slug.as_str()).collect();
    let mut tag_slugs: BTreeSet<String> = BTreeSet::new();

    for tag in site_tags {
        let slugified = slugify(&tag.display);
        let base_slug = if slugified.is_empty() {
            "tag".to_owned()
        } else {
            format!("tag-{slugified}")
        };

        let mut slug = base_slug.clone();
        let mut suffix = 2;
        while page_heading_slugs.contains(slug.as_str()) || tag_slugs.contains(&slug) {
            slug = format!("{base_slug}-{suffix}");
            suffix += 1;
        }
        tag_slugs.insert(slug.clone());

        let heading_node = PageNode::Heading {
            level: 2,
            slug: slug.clone(),
            children: vec![PageNode::Text(tag.display.clone())],
        };

        let mut items = Vec::with_capacity(tag.pages.len());
        for (page_id, page_title) in tag.pages {
            let link_index = state.links.len();
            state.links.push(LinkTarget::Page {
                id: page_id,
                anchor: None,
            });
            items.push(vec![PageNode::WikiLink {
                index: link_index,
                children: vec![PageNode::Text(page_title)],
            }]);
        }
        let list_node = PageNode::List { start: None, items };

        toc.push(TocEntry {
            level: 2,
            text: tag.display,
            anchor: slug,
        });

        body.push(heading_node);
        body.push(list_node);
    }
}

struct AssemblyState<'a> {
    host: &'a PageId,
    documents: &'a [Document],
    graph: &'a SiteGraph,
    links: Vec<LinkTarget>,
    used_slugs: BTreeSet<String>,
}

impl AssemblyState<'_> {
    fn convert_nodes(
        &mut self,
        nodes: &[DocumentNode],
        origin: &Document,
        region: &TransclusionRegion,
        imported: bool,
        anchors: &BTreeMap<String, String>,
        chain: &[usize],
    ) -> Vec<PageNode> {
        let mut output = Vec::new();
        for node in nodes {
            output.extend(self.convert_node(node, origin, region, imported, anchors, chain));
        }
        output
    }

    fn convert_node(
        &mut self,
        node: &DocumentNode,
        origin: &Document,
        region: &TransclusionRegion,
        imported: bool,
        anchors: &BTreeMap<String, String>,
        chain: &[usize],
    ) -> Vec<PageNode> {
        match node {
            DocumentNode::Heading {
                level,
                slug,
                children,
            } => vec![PageNode::Heading {
                level: *level,
                slug: if imported {
                    anchors.get(slug).cloned().unwrap_or_else(|| slug.clone())
                } else {
                    slug.clone()
                },
                children: self.convert_nodes(children, origin, region, imported, anchors, chain),
            }],
            DocumentNode::Paragraph(children) => vec![PageNode::Paragraph(
                self.convert_nodes(children, origin, region, imported, anchors, chain),
            )],
            DocumentNode::Text(text) => vec![PageNode::Text(text.clone())],
            DocumentNode::Code(text) => vec![PageNode::Code(text.clone())],
            DocumentNode::CodeBlock { lang, text } => vec![PageNode::CodeBlock {
                lang: lang.clone(),
                text: text.clone(),
            }],
            DocumentNode::Emphasis(children) => vec![PageNode::Emphasis(
                self.convert_nodes(children, origin, region, imported, anchors, chain),
            )],
            DocumentNode::Strong(children) => vec![PageNode::Strong(
                self.convert_nodes(children, origin, region, imported, anchors, chain),
            )],
            DocumentNode::Strikethrough(children) => {
                vec![PageNode::Strikethrough(self.convert_nodes(
                    children, origin, region, imported, anchors, chain,
                ))]
            }
            DocumentNode::Link { href, children } => vec![PageNode::Link {
                href: if imported {
                    rebase_uri(self.host, origin_id(origin), href, false, anchors)
                } else {
                    href.clone()
                },
                children: self.convert_nodes(children, origin, region, imported, anchors, chain),
            }],
            DocumentNode::WikiLink { index, children } => {
                let index = if imported {
                    self.append_imported_link(origin, *index)
                } else {
                    *index
                };
                vec![PageNode::WikiLink {
                    index,
                    children: self
                        .convert_nodes(children, origin, region, imported, anchors, chain),
                }]
            }
            DocumentNode::Image { src, alt } => vec![PageNode::Image {
                src: if imported {
                    rebase_uri(self.host, origin_id(origin), src, true, anchors)
                } else {
                    src.clone()
                },
                alt: alt.clone(),
            }],
            DocumentNode::List { start, items } => vec![PageNode::List {
                start: *start,
                items: items
                    .iter()
                    .map(|item| self.convert_nodes(item, origin, region, imported, anchors, chain))
                    .collect(),
            }],
            DocumentNode::BlockQuote { kind, children } => vec![PageNode::BlockQuote {
                kind: *kind,
                children: self.convert_nodes(children, origin, region, imported, anchors, chain),
            }],
            DocumentNode::Table { align, head, rows } => vec![PageNode::Table {
                align: align.clone(),
                head: head
                    .iter()
                    .map(|cell| self.convert_nodes(cell, origin, region, imported, anchors, chain))
                    .collect(),
                rows: rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|cell| {
                                self.convert_nodes(cell, origin, region, imported, anchors, chain)
                            })
                            .collect()
                    })
                    .collect(),
            }],
            DocumentNode::Rule => vec![PageNode::Rule],
            DocumentNode::SoftBreak => vec![PageNode::SoftBreak],
            DocumentNode::HardBreak => vec![PageNode::HardBreak],
            DocumentNode::Html(text) => vec![PageNode::Html(text.clone())],
            DocumentNode::TaskMarker(checked) => vec![PageNode::TaskMarker(*checked)],
            DocumentNode::Anchor { id } => {
                if imported {
                    vec![]
                } else {
                    vec![PageNode::Anchor { id: id.clone() }]
                }
            }
            DocumentNode::Transclusion(transclusion) => {
                self.expand_transclusion(origin, region, transclusion, chain)
            }
        }
    }

    fn expand_transclusion(
        &mut self,
        _source: &Document,
        source_region: &TransclusionRegion,
        transclusion: &document::Transclusion,
        chain: &[usize],
    ) -> Vec<PageNode> {
        let occurrence = TransclusionOccurrence {
            source: source_region.clone(),
            transclusion_id: transclusion.id,
        };
        if let Some(cycle) = self.graph.blocked_transclusion(&occurrence) {
            return vec![PageNode::Paragraph(vec![PageNode::Text(cycle_marker(
                cycle,
            ))])];
        }

        let Some(target_region) = self.graph.transclusion_target(&occurrence).cloned() else {
            return unavailable(transclusion.display.as_str());
        };
        let Some(target_document) = self
            .documents
            .iter()
            .find(|document| document.id == target_region.page)
            .cloned()
        else {
            return unavailable(transclusion.display.as_str());
        };
        let Some(fragment) = section_nodes(&target_document, target_region.anchor.as_deref())
        else {
            return unavailable(transclusion.display.as_str());
        };

        let mut nested_chain = chain.to_vec();
        nested_chain.push(transclusion.id);
        let fragment_anchors = allocate_fragment(&fragment, &nested_chain, &mut self.used_slugs);
        self.convert_nodes(
            &fragment,
            &target_document,
            &target_region,
            true,
            &fragment_anchors,
            &nested_chain,
        )
    }

    fn append_imported_link(&mut self, origin: &Document, index: usize) -> usize {
        let target = origin
            .links
            .get(index)
            .and_then(|_| self.graph.resolution(&origin.id, index))
            .map(|resolved| LinkTarget::Page {
                id: resolved.to.clone(),
                anchor: resolved.anchor.clone(),
            })
            .unwrap_or(LinkTarget::Unresolved);
        self.links.push(target);
        self.links.len() - 1
    }
}

fn origin_id(document: &Document) -> &PageId {
    &document.id
}

fn unavailable(display: &str) -> Vec<PageNode> {
    vec![PageNode::Paragraph(vec![PageNode::Text(format!(
        "[transclusion unavailable: {display}]"
    ))])]
}

fn cycle_marker(cycle: &TransclusionCycle) -> String {
    let path = cycle
        .path
        .iter()
        .map(region_label)
        .collect::<Vec<_>>()
        .join(" -> ");
    format!("[transclusion cycle: {path}]")
}

fn region_label(region: &TransclusionRegion) -> String {
    match &region.anchor {
        Some(anchor) => format!("{}#{anchor}", region.page),
        None => region.page.to_string(),
    }
}

fn collect_heading_slugs(nodes: &[DocumentNode], used: &mut BTreeSet<String>) {
    for node in nodes {
        match node {
            DocumentNode::Heading { slug, children, .. } => {
                used.insert(slug.clone());
                collect_heading_slugs(children, used);
            }
            DocumentNode::Paragraph(children)
            | DocumentNode::Emphasis(children)
            | DocumentNode::Strong(children)
            | DocumentNode::Strikethrough(children)
            | DocumentNode::Link { children, .. }
            | DocumentNode::WikiLink { children, .. }
            | DocumentNode::BlockQuote { children, .. } => {
                collect_heading_slugs(children, used);
            }
            DocumentNode::List { items, .. } => {
                for item in items {
                    collect_heading_slugs(item, used);
                }
            }
            DocumentNode::Table { head, rows, .. } => {
                for cell in head {
                    collect_heading_slugs(cell, used);
                }
                for row in rows {
                    for cell in row {
                        collect_heading_slugs(cell, used);
                    }
                }
            }
            DocumentNode::Text(_)
            | DocumentNode::Code(_)
            | DocumentNode::CodeBlock { .. }
            | DocumentNode::Image { .. }
            | DocumentNode::Rule
            | DocumentNode::SoftBreak
            | DocumentNode::HardBreak
            | DocumentNode::Html(_)
            | DocumentNode::TaskMarker(_)
            | DocumentNode::Anchor { .. }
            | DocumentNode::Transclusion(_) => {}
        }
    }
}

fn allocate_fragment(
    nodes: &[DocumentNode],
    chain: &[usize],
    used: &mut BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut slugs = Vec::new();
    collect_fragment_headings(nodes, &mut slugs);
    let prefix = chain
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("-");
    let mut anchors = BTreeMap::new();
    for slug in slugs {
        let base = format!("tx-{prefix}-{slug}");
        let mut candidate = base.clone();
        let mut suffix = 2;
        while used.contains(&candidate) {
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
        used.insert(candidate.clone());
        anchors.insert(slug, candidate);
    }
    anchors
}

fn collect_fragment_headings(nodes: &[DocumentNode], slugs: &mut Vec<String>) {
    for node in nodes {
        match node {
            DocumentNode::Heading { slug, children, .. } => {
                slugs.push(slug.clone());
                collect_fragment_headings(children, slugs);
            }
            DocumentNode::Paragraph(children)
            | DocumentNode::Emphasis(children)
            | DocumentNode::Strong(children)
            | DocumentNode::Strikethrough(children)
            | DocumentNode::Link { children, .. }
            | DocumentNode::WikiLink { children, .. }
            | DocumentNode::BlockQuote { children, .. } => {
                collect_fragment_headings(children, slugs);
            }
            DocumentNode::List { items, .. } => {
                for item in items {
                    collect_fragment_headings(item, slugs);
                }
            }
            DocumentNode::Table { head, rows, .. } => {
                for cell in head {
                    collect_fragment_headings(cell, slugs);
                }
                for row in rows {
                    for cell in row {
                        collect_fragment_headings(cell, slugs);
                    }
                }
            }
            DocumentNode::Text(_)
            | DocumentNode::Code(_)
            | DocumentNode::CodeBlock { .. }
            | DocumentNode::Image { .. }
            | DocumentNode::Rule
            | DocumentNode::SoftBreak
            | DocumentNode::HardBreak
            | DocumentNode::Html(_)
            | DocumentNode::TaskMarker(_)
            | DocumentNode::Anchor { .. }
            | DocumentNode::Transclusion(_) => {}
        }
    }
}

fn rebase_uri(
    host: &PageId,
    origin: &PageId,
    destination: &str,
    is_image: bool,
    anchors: &BTreeMap<String, String>,
) -> String {
    if is_scheme_reference(destination) {
        return destination.to_owned();
    }

    let encoded_host = percent_encode(host.as_str(), true);
    let encoded_origin = percent_encode(origin.as_str(), true);
    let host_directory = encoded_host
        .split('/')
        .take(encoded_host.matches('/').count())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let origin_directory = encoded_origin
        .split('/')
        .take(encoded_origin.matches('/').count())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let (path, suffix) = split_suffix(destination);

    if !is_image
        && destination.starts_with('#')
        && path.is_empty()
        && let Some(decoded) = decode_fragment(&suffix[1..])
        && let Some(mapped) = anchors.get(&decoded)
    {
        return format!("#{}", percent_encode(mapped, false));
    }

    let (target, trailing_slash) = if path.is_empty() {
        (
            origin_directory
                .iter()
                .cloned()
                .chain(std::iter::once(format!(
                    "{}.md",
                    encoded_origin.rsplit('/').next().unwrap_or_default()
                )))
                .collect(),
            false,
        )
    } else {
        let mut components = origin_directory;
        components.extend(path.split('/').map(str::to_owned));
        (normalize_components(components), path.ends_with('/'))
    };
    let rebased = relative_path(&target, &host_directory, trailing_slash);
    format!("{rebased}{suffix}")
}

fn percent_encode(value: &str, preserve_slash: bool) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(*byte, b'-' | b'.' | b'_' | b'~')
            || (preserve_slash && *byte == b'/')
        {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn is_scheme_reference(value: &str) -> bool {
    if value.starts_with('/') {
        return true;
    }
    let bytes = value.as_bytes();
    if bytes.first().is_none_or(|byte| !byte.is_ascii_alphabetic()) {
        return false;
    }
    let mut index = 1;
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'+' | b'-' | b'.'))
    {
        index += 1;
    }
    if index >= bytes.len() || bytes[index] != b':' {
        return false;
    }
    value.find('/').is_none_or(|separator| index < separator)
}

fn split_suffix(value: &str) -> (&str, &str) {
    let question = value.find('?');
    let fragment = value.find('#');
    let split = match (question, fragment) {
        (Some(question), Some(fragment)) => Some(question.min(fragment)),
        (Some(position), None) | (None, Some(position)) => Some(position),
        (None, None) => None,
    };
    split.map_or((value, ""), |position| value.split_at(position))
}

fn decode_fragment(fragment: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let source = fragment.as_bytes();
    let mut index = 0;
    while index < source.len() {
        if source[index] == b'%' {
            if index + 2 >= source.len() {
                return None;
            }
            let high = hex_value(source[index + 1])?;
            let low = hex_value(source[index + 2])?;
            bytes.push((high << 4) | low);
            index += 3;
        } else {
            let character = fragment[index..].chars().next()?;
            let mut encoded = [0; 4];
            bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            index += character.len_utf8();
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn normalize_components(components: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for component in components {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            if normalized.last().is_some_and(|last| last != "..") {
                normalized.pop();
            } else {
                normalized.push(component);
            }
        } else {
            normalized.push(component);
        }
    }
    normalized
}

fn relative_path(target: &[String], base: &[String], trailing_slash: bool) -> String {
    let mut common = 0;
    while common < target.len() && common < base.len() && target[common] == base[common] {
        common += 1;
    }
    let mut components = vec!["..".to_owned(); base.len() - common];
    components.extend(target[common..].iter().cloned());
    let mut result = if components.is_empty() {
        ".".to_owned()
    } else {
        components.join("/")
    };
    if trailing_slash {
        result.push('/');
    }
    let first = result.split('/').next().unwrap_or_default();
    if is_scheme_reference(first) {
        result.insert_str(0, "./");
    }
    result
}

#[cfg(test)]
mod transclusion_tests;

#[cfg(test)]
mod tag_index_tests {
    use super::*;
    use document::{FrontMatter, Heading};
    use library::Digest;

    fn make_test_doc(
        id: &str,
        title: &str,
        tags: Vec<&str>,
        headings: Vec<(&str, &str)>,
        index: Option<&str>,
    ) -> Document {
        Document {
            id: PageId::new(id),
            source_digest: Digest::of(id.as_bytes()),
            title: title.to_owned(),
            headings: headings
                .into_iter()
                .map(|(text, slug)| Heading {
                    level: 2,
                    text: text.to_owned(),
                    slug: slug.to_owned(),
                    base_slug: slug.to_owned(),
                    line: 1,
                })
                .collect(),
            links: Vec::new(),
            body: Vec::new(),
            diagnostics: Vec::new(),
            section_refs: Vec::new(),
            reference_entries: Vec::new(),
            citations: Vec::new(),
            front_matter: Some(FrontMatter {
                title: Some(title.to_owned()),
                tags: tags.into_iter().map(String::from).collect(),
                end_line: 5,
                index: index.map(String::from),
            }),
        }
    }

    #[test]
    fn collect_site_tags_ordering_and_case() {
        let doc1 = make_test_doc("p1", "Page One", vec!["Alpha", "BETA"], vec![], None);
        let doc2 = make_test_doc(
            "p2",
            "Page Two",
            vec!["alpha", "beta", "gamma"],
            vec![],
            None,
        );
        let doc0 = make_test_doc("p0", "Page Zero", vec!["ALPHA", "Delta"], vec![], None);

        // Intentionally provide unordered
        let docs = vec![doc1, doc2, doc0];
        let site_tags = collect_site_tags(&docs);

        assert_eq!(site_tags.len(), 4);

        // Tag order: ALPHA, BETA, Delta, gamma
        assert_eq!(site_tags[0].display, "ALPHA"); // from p0
        assert_eq!(
            site_tags[0].pages,
            vec![
                (PageId::new("p0"), "Page Zero".to_owned()),
                (PageId::new("p1"), "Page One".to_owned()),
                (PageId::new("p2"), "Page Two".to_owned()),
            ]
        );

        assert_eq!(site_tags[1].display, "BETA"); // from p1
        assert_eq!(
            site_tags[1].pages,
            vec![
                (PageId::new("p1"), "Page One".to_owned()),
                (PageId::new("p2"), "Page Two".to_owned()),
            ]
        );

        assert_eq!(site_tags[2].display, "Delta"); // from p0
        assert_eq!(
            site_tags[2].pages,
            vec![(PageId::new("p0"), "Page Zero".to_owned())]
        );

        assert_eq!(site_tags[3].display, "gamma"); // from p2
        assert_eq!(
            site_tags[3].pages,
            vec![(PageId::new("p2"), "Page Two".to_owned())]
        );
    }

    #[test]
    fn append_tag_index_sections_slug_collisions() {
        let index_doc = make_test_doc(
            "index",
            "Tag Index",
            vec![],
            vec![
                ("Tag Foo", "tag-foo"),
                ("Tag Foo 2", "tag-foo-2"),
                ("Tag", "tag"),
            ],
            Some("tags"),
        );
        let doc_a = make_test_doc("a", "Doc A", vec!["foo", "foo!", "😀"], vec![], None);

        let docs = vec![index_doc.clone(), doc_a];
        let graph = graph::resolve(&docs);
        let mut state = AssemblyState {
            host: &index_doc.id,
            documents: &docs,
            graph: &graph,
            links: Vec::new(),
            used_slugs: BTreeSet::new(),
        };
        let mut body = Vec::new();
        let mut toc = Vec::new();

        append_tag_index_sections(&index_doc, &docs, &mut state, &mut body, &mut toc);

        // 3 tags: "foo", "foo!", "😀"
        // foo: slugified is "foo", base "tag-foo", collides with tag-foo and tag-foo-2 -> tag-foo-3
        // foo!: slugified is "foo", base "tag-foo", collides with tag-foo, tag-foo-2, tag-foo-3 -> tag-foo-4
        // 😀: slugified is "", base "tag", collides with tag -> tag-2
        assert_eq!(toc.len(), 3);
        assert_eq!(toc[0].text, "foo");
        assert_eq!(toc[0].anchor, "tag-foo-3");
        assert_eq!(toc[1].text, "foo!");
        assert_eq!(toc[1].anchor, "tag-foo-4");
        assert_eq!(toc[2].text, "😀");
        assert_eq!(toc[2].anchor, "tag-2");

        assert_eq!(body.len(), 6);
        assert_eq!(
            body[0],
            PageNode::Heading {
                level: 2,
                slug: "tag-foo-3".to_owned(),
                children: vec![PageNode::Text("foo".to_owned())],
            }
        );
        assert_eq!(
            body[2],
            PageNode::Heading {
                level: 2,
                slug: "tag-foo-4".to_owned(),
                children: vec![PageNode::Text("foo!".to_owned())],
            }
        );
        assert_eq!(
            body[4],
            PageNode::Heading {
                level: 2,
                slug: "tag-2".to_owned(),
                children: vec![PageNode::Text("😀".to_owned())],
            }
        );
    }

    #[test]
    fn empty_site_tags_appends_nothing() {
        let index_doc = make_test_doc("index", "Tag Index", vec![], vec![], Some("tags"));
        let other_doc = make_test_doc("other", "Other", vec![], vec![], None);

        let docs = vec![index_doc.clone(), other_doc];
        let graph = graph::resolve(&docs);
        let mut state = AssemblyState {
            host: &index_doc.id,
            documents: &docs,
            graph: &graph,
            links: Vec::new(),
            used_slugs: BTreeSet::new(),
        };
        let mut body = Vec::new();
        let mut toc = Vec::new();

        append_tag_index_sections(&index_doc, &docs, &mut state, &mut body, &mut toc);

        assert!(body.is_empty());
        assert!(toc.is_empty());
        assert!(state.links.is_empty());
    }
}
