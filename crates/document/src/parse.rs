use std::collections::BTreeMap;

use library::{PageId, Source};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag,
    TagEnd,
};

use crate::{
    Align, CalloutKind, Diagnostic, Document, Heading, Link, ReferenceEntry, Transclusion,
    citation::{PendingCitation, parse_citation_label, resolve_citations},
    section_ref::{PendingRef, ScannedPiece, resolve_section_refs, scan_references},
    slugify,
};

/// The parser options. `ENABLE_HEADING_ATTRIBUTES` is off on purpose: it
/// strips `{…}` from heading text, and the registered contract derives a
/// slug from the text as written (CHG-005 decision `heading-attributes-off`).
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_WIKILINKS
        | Options::ENABLE_GFM
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

/// An open container while the tree is built.
enum Frame {
    Heading {
        level: u8,
        line: usize,
        text: String,
    },
    Paragraph,
    Emphasis,
    Strong,
    Strike,
    Link {
        href: String,
    },
    WikiLink {
        index: usize,
    },
    Image {
        src: String,
        alt: String,
        line: usize,
        wikilink: bool,
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<BuildNode>>,
    },
    Item,
    Quote {
        kind: Option<CalloutKind>,
    },
    CodeBlock {
        lang: Option<String>,
        text: String,
    },
    Table {
        align: Vec<Align>,
        head: Vec<Vec<BuildNode>>,
        rows: Vec<Vec<Vec<BuildNode>>>,
        in_head: bool,
    },
    Row {
        cells: Vec<Vec<BuildNode>>,
    },
    Cell,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BuildNode {
    Heading {
        level: u8,
        slug: String,
        children: Vec<BuildNode>,
    },
    Paragraph(Vec<BuildNode>),
    Text(String),
    Code(String),
    CodeBlock {
        lang: Option<String>,
        text: String,
    },
    Emphasis(Vec<BuildNode>),
    Strong(Vec<BuildNode>),
    Strikethrough(Vec<BuildNode>),
    Link {
        href: String,
        children: Vec<BuildNode>,
    },
    WikiLink {
        index: usize,
        children: Vec<BuildNode>,
    },
    Transclusion(Transclusion),
    Image {
        src: String,
        alt: String,
        line: usize,
        wikilink: bool,
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<BuildNode>>,
    },
    BlockQuote {
        kind: Option<CalloutKind>,
        children: Vec<BuildNode>,
    },
    Table {
        align: Vec<Align>,
        head: Vec<Vec<BuildNode>>,
        rows: Vec<Vec<Vec<BuildNode>>>,
    },
    Rule,
    SoftBreak,
    HardBreak,
    Html(String),
    TaskMarker(bool),
    SectionRef(usize),
    Anchor {
        id: String,
    },
    Citation(usize),
    OpenBracket {
        offset: usize,
        line: usize,
        /// Text that preceded `[` in the same pulldown-cmark text event.
        /// When the bracket does not form a citation, it is rejoined with the
        /// bracket: `Text(prefix + "[")`.
        prefix: Option<String>,
    },
}

impl BuildNode {
    fn is_ignorable(&self) -> bool {
        matches!(
            self,
            Self::Text(text)
                if text.chars().all(|character| matches!(character, ' ' | '\t'))
        )
    }
}

fn level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn callout(kind: BlockQuoteKind) -> CalloutKind {
    match kind {
        BlockQuoteKind::Note => CalloutKind::Note,
        BlockQuoteKind::Tip => CalloutKind::Tip,
        BlockQuoteKind::Important => CalloutKind::Important,
        BlockQuoteKind::Warning => CalloutKind::Warning,
        BlockQuoteKind::Caution => CalloutKind::Caution,
    }
}

fn align(a: Alignment) -> Align {
    match a {
        Alignment::None => Align::None,
        Alignment::Left => Align::Left,
        Alignment::Center => Align::Center,
        Alignment::Right => Align::Right,
    }
}

/// Line starts, for byte offset to 1-based line.
fn line_of(starts: &[usize], offset: usize) -> usize {
    starts.partition_point(|&s| s <= offset)
}

struct Builder {
    stack: Vec<(Frame, Vec<BuildNode>)>,
    headings: Vec<Heading>,
    links: Vec<Link>,
    diagnostics: Vec<Diagnostic>,
    seen: BTreeMap<String, (usize, usize)>,
    /// Every final slug allocated so far on the page.
    used: std::collections::BTreeSet<String>,
    next_transclusion: usize,
    pending_refs: Vec<PendingRef>,
    reference_entries: Vec<ReferenceEntry>,
    seen_entries: BTreeMap<String, usize>,
    pending_citations: Vec<PendingCitation>,
}

impl Builder {
    fn push_node(&mut self, node: BuildNode) {
        if let Some((_, children)) = self.stack.last_mut() {
            children.push(node);
        }
    }

    fn transclusion(&mut self, children: &[BuildNode]) -> Option<Transclusion> {
        let mut meaningful = children.iter().filter(|node| !node.is_ignorable());
        let Some(BuildNode::Image {
            src,
            line,
            wikilink: true,
            ..
        }) = meaningful.next()
        else {
            return None;
        };
        if meaningful.next().is_some() {
            return None;
        }

        let (target, anchor) = match src.split_once('#') {
            Some((target, anchor)) => (target, Some(anchor)),
            None => (src.as_str(), None),
        };
        if target.is_empty() || anchor == Some("") {
            return None;
        }

        let id = self.next_transclusion;
        self.next_transclusion += 1;
        Some(Transclusion {
            id,
            target: target.to_owned(),
            anchor: anchor.map(str::to_owned),
            display: src.clone(),
            line: *line,
        })
    }

    /// Plain text for an enclosing heading, if any.
    fn heading_text(&mut self, text: &str) {
        for (frame, _) in self.stack.iter_mut().rev() {
            if let Frame::Heading { text: t, .. } = frame {
                t.push_str(text);
                return;
            }
        }
    }

    fn check_reference_entry(&mut self, children: &mut Vec<BuildNode>) {
        if children.is_empty() {
            return;
        }
        let BuildNode::Citation(cit_idx) = children[0] else {
            return;
        };
        let is_entry = if children.len() == 1 {
            true
        } else {
            match &children[1] {
                BuildNode::Text(s) => s.starts_with(' '),
                // A break, or any block: in a tight list item the item's
                // inline text ends where a nested block begins, so the label
                // is "followed by the end of the paragraph" (contract section
                // 1). An HTML block arrives as a Paragraph.
                BuildNode::SoftBreak
                | BuildNode::HardBreak
                | BuildNode::Heading { .. }
                | BuildNode::Paragraph(_)
                | BuildNode::CodeBlock { .. }
                | BuildNode::List { .. }
                | BuildNode::BlockQuote { .. }
                | BuildNode::Table { .. }
                | BuildNode::Rule
                | BuildNode::Transclusion(_) => true,
                _ => false,
            }
        };
        if !is_entry {
            return;
        }

        let cit = &mut self.pending_citations[cit_idx];
        let label = cit.label.clone();
        let line = cit.line;

        if let Some(&first_line) = self.seen_entries.get(&label) {
            self.diagnostics.push(Diagnostic::DuplicateReferenceEntry {
                label,
                first_line,
                second_line: line,
            });
        } else {
            self.seen_entries.insert(label.clone(), line);
            let anchor = format!("ref-{}", label.to_ascii_lowercase());
            self.reference_entries.push(ReferenceEntry {
                label,
                anchor: anchor.clone(),
                line,
            });
            cit.is_entry_label = true;
            children.insert(0, BuildNode::Anchor { id: anchor });
        }
    }

    fn start(&mut self, tag: Tag<'_>, line: usize) {
        let frame = match tag {
            Tag::Heading { level: l, .. } => Frame::Heading {
                level: level(l),
                line,
                text: String::new(),
            },
            Tag::Paragraph => Frame::Paragraph,
            Tag::Emphasis => Frame::Emphasis,
            Tag::Strong => Frame::Strong,
            Tag::Strikethrough => Frame::Strike,
            Tag::Link {
                link_type: LinkType::WikiLink { has_pothole },
                dest_url,
                ..
            } => {
                let dest = dest_url.to_string();
                let (target, anchor) = match dest.split_once('#') {
                    Some((t, a)) => (t.to_owned(), Some(a.to_owned())),
                    None => (dest, None),
                };
                self.links.push(Link {
                    target,
                    anchor,
                    alias: has_pothole.then(String::new),
                    line,
                });
                Frame::WikiLink {
                    index: self.links.len() - 1,
                }
            }
            Tag::Link { dest_url, .. } => Frame::Link {
                href: dest_url.to_string(),
            },
            Tag::Image {
                link_type,
                dest_url,
                ..
            } => Frame::Image {
                src: dest_url.to_string(),
                alt: String::new(),
                line,
                wikilink: matches!(link_type, LinkType::WikiLink { has_pothole: false }),
            },
            Tag::List(start) => Frame::List {
                start,
                items: Vec::new(),
            },
            Tag::Item => Frame::Item,
            Tag::BlockQuote(kind) => Frame::Quote {
                kind: kind.map(callout),
            },
            Tag::CodeBlock(kind) => Frame::CodeBlock {
                lang: match kind {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_owned();
                        (!lang.is_empty()).then_some(lang)
                    }
                    CodeBlockKind::Indented => None,
                },
                text: String::new(),
            },
            Tag::Table(a) => Frame::Table {
                align: a.into_iter().map(align).collect(),
                head: Vec::new(),
                rows: Vec::new(),
                in_head: false,
            },
            Tag::TableHead => {
                if let Some((Frame::Table { in_head, .. }, _)) = self.stack.last_mut() {
                    *in_head = true;
                }
                Frame::Row { cells: Vec::new() }
            }
            Tag::TableRow => Frame::Row { cells: Vec::new() },
            Tag::TableCell => Frame::Cell,
            Tag::HtmlBlock => {
                self.diagnostics
                    .push(Diagnostic::Unsupported { kind: "html", line });
                Frame::Other
            }
            _ => {
                self.diagnostics.push(Diagnostic::Unsupported {
                    kind: "block",
                    line,
                });
                Frame::Other
            }
        };
        self.stack.push((frame, Vec::new()));
    }

    fn end(&mut self, _end: TagEnd) {
        let Some((frame, children)) = self.stack.pop() else {
            return;
        };
        let node = match frame {
            Frame::Heading { level, line, text } => {
                let base = slugify(&text);
                // Final slugs are unique on the page. A repeated base gets the
                // next `-k` (k from 2) that no heading has taken, and a natural
                // slug that collides with an allocated one advances the same
                // way; only a repeated BASE is a DuplicateSlug witness, as the
                // registered contract says (review finding 3: `# A`, `# A`,
                // `# A-2` gave `a`, `a-2`, `a-2`).
                let next_free = |used: &std::collections::BTreeSet<String>, from: usize| {
                    (from..)
                        .map(|k| format!("{base}-{k}"))
                        .find(|c| !used.contains(c))
                        .unwrap_or_default()
                };
                let slug = match self.seen.get_mut(&base) {
                    Some((count, first_line)) => {
                        *count += 1;
                        self.diagnostics.push(Diagnostic::DuplicateSlug {
                            slug: base.clone(),
                            first_line: *first_line,
                            second_line: line,
                        });
                        next_free(&self.used, *count)
                    }
                    None => {
                        self.seen.insert(base.clone(), (1, line));
                        if self.used.contains(&base) {
                            next_free(&self.used, 2)
                        } else {
                            base.clone()
                        }
                    }
                };
                self.used.insert(slug.clone());
                self.headings.push(Heading {
                    level,
                    text,
                    slug: slug.clone(),
                    base_slug: base,
                    line,
                });
                Some(BuildNode::Heading {
                    level,
                    slug,
                    children,
                })
            }
            Frame::Paragraph => {
                if let Some(transclusion) = self.transclusion(&children) {
                    Some(BuildNode::Transclusion(transclusion))
                } else {
                    Some(BuildNode::Paragraph(children))
                }
            }
            Frame::Emphasis => Some(BuildNode::Emphasis(children)),
            Frame::Strong => Some(BuildNode::Strong(children)),
            Frame::Strike => Some(BuildNode::Strikethrough(children)),
            Frame::Link { href } => Some(BuildNode::Link { href, children }),
            Frame::WikiLink { index } => {
                if let Some(link) = self.links.get_mut(index)
                    && link.alias.is_some()
                {
                    link.alias = Some(plain(&children));
                }
                Some(BuildNode::WikiLink { index, children })
            }
            Frame::Image {
                src,
                alt,
                line,
                wikilink,
            } => Some(BuildNode::Image {
                src,
                alt,
                line,
                wikilink,
            }),
            Frame::List { start, items } => Some(BuildNode::List { start, items }),
            Frame::Item => {
                let item = if let Some(transclusion) = self.transclusion(&children) {
                    vec![BuildNode::Transclusion(transclusion)]
                } else {
                    children
                };
                if let Some((Frame::List { items, .. }, _)) = self.stack.last_mut() {
                    items.push(item);
                }
                None
            }
            Frame::Quote { kind } => Some(BuildNode::BlockQuote { kind, children }),
            Frame::CodeBlock { lang, text } => Some(BuildNode::CodeBlock { lang, text }),
            Frame::Table {
                align, head, rows, ..
            } => Some(BuildNode::Table { align, head, rows }),
            Frame::Row { cells } => {
                if let Some((
                    Frame::Table {
                        head,
                        rows,
                        in_head,
                        ..
                    },
                    _,
                )) = self.stack.last_mut()
                {
                    if *in_head {
                        *head = cells;
                        *in_head = false;
                    } else {
                        rows.push(cells);
                    }
                }
                None
            }
            Frame::Cell => {
                if let Some((Frame::Row { cells }, _)) = self.stack.last_mut() {
                    cells.push(children);
                }
                None
            }
            Frame::Other => Some(BuildNode::Paragraph(children)),
        };
        if let Some(node) = node {
            self.push_node(node);
        }
    }

    fn is_reference_excluded(&self) -> bool {
        self.stack.iter().skip(1).any(|(frame, _)| {
            matches!(
                frame,
                Frame::Heading { .. }
                    | Frame::Link { .. }
                    | Frame::WikiLink { .. }
                    | Frame::Image { .. }
                    | Frame::CodeBlock { .. }
                    | Frame::Other
            )
        })
    }

    fn try_match_citation(children: &[BuildNode], offset: usize) -> Option<(String, usize)> {
        if children.len() < 2 {
            return None;
        }
        let n = children.len();
        let BuildNode::OpenBracket {
            offset: open_offset,
            line: open_line,
            ..
        } = children[n - 2]
        else {
            return None;
        };
        let BuildNode::Text(ref label_str) = children[n - 1] else {
            return None;
        };
        if open_offset + 1 + label_str.len() != offset {
            return None;
        }
        let label = parse_citation_label(label_str)?;
        Some((label.to_owned(), open_line))
    }

    fn text(&mut self, text: &str, offset: usize, starts: &[usize]) {
        match self.stack.last_mut() {
            Some((Frame::CodeBlock { text: t, .. }, _)) => {
                t.push_str(text);
                return;
            }
            Some((Frame::Image { alt, .. }, _)) => {
                alt.push_str(text);
                return;
            }
            _ => {}
        }
        self.heading_text(text);
        if self.is_reference_excluded() {
            self.push_node(BuildNode::Text(text.to_owned()));
            return;
        }

        if let Some(prefix) = text.strip_suffix('[') {
            let line = line_of(starts, offset + prefix.len());
            self.push_node(BuildNode::OpenBracket {
                offset: offset + prefix.len(),
                line,
                prefix: if prefix.is_empty() {
                    None
                } else {
                    Some(prefix.to_owned())
                },
            });
            return;
        }

        if text == "]" {
            let matched = self
                .stack
                .last_mut()
                .and_then(|(_, children)| Self::try_match_citation(children, offset));
            if let Some((label, open_line)) = matched {
                let cit_idx = self.pending_citations.len();
                self.pending_citations.push(PendingCitation {
                    label,
                    line: open_line,
                    is_entry_label: false,
                });
                if let Some((_, children)) = self.stack.last_mut() {
                    children.pop(); // label text
                    // Extract prefix from the OpenBracket before removing it.
                    let prefix = match children.pop() {
                        Some(BuildNode::OpenBracket { prefix, .. }) => prefix,
                        _ => None,
                    };
                    if let Some(p) = prefix {
                        children.push(BuildNode::Text(p));
                    }
                    children.push(BuildNode::Citation(cit_idx));
                }
                return;
            }
            self.push_node(BuildNode::Text("]".to_owned()));
            return;
        }

        if !text.contains('§') {
            self.push_node(BuildNode::Text(text.to_owned()));
            return;
        }

        let pieces = scan_references(text, offset, starts);
        for piece in pieces {
            match piece {
                ScannedPiece::Text(t) => {
                    self.push_node(BuildNode::Text(t));
                }
                ScannedPiece::Ref {
                    ref_text,
                    number,
                    line,
                } => {
                    let ref_index = self.pending_refs.len();
                    self.pending_refs.push(PendingRef {
                        text: ref_text,
                        number,
                        line,
                    });
                    self.push_node(BuildNode::SectionRef(ref_index));
                }
            }
        }
    }
}

/// Walk `nodes` in document order and register reference entries.
///
/// This runs after the full tree is built so that entries appear in source
/// order regardless of nesting depth. The contract says "the first entry with
/// a given label is that label's entry" and "`reference_entries` in document
/// order"; walking the finished tree guarantees both.
fn register_entries(nodes: &mut [BuildNode], b: &mut Builder) {
    for node in nodes.iter_mut() {
        match node {
            BuildNode::Paragraph(children) => {
                b.check_reference_entry(children);
                // Do not recurse into paragraph children: they are inline nodes
                // and cannot contain further paragraphs or list items.
            }
            BuildNode::List { items, .. } => {
                for item in items.iter_mut() {
                    // Check entry for this item first (the item's own leading
                    // citation), then recurse into the item's children for
                    // nested blocks. This guarantees outer-before-inner order.
                    b.check_reference_entry(item);
                    register_entries(item, b);
                }
            }
            BuildNode::BlockQuote { children, .. } => {
                register_entries(children, b);
            }
            _ => {}
        }
    }
}

/// The plain text of a node list.
fn plain(nodes: &[BuildNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            BuildNode::Text(text) | BuildNode::Code(text) => out.push_str(text),
            BuildNode::Emphasis(children)
            | BuildNode::Strong(children)
            | BuildNode::Strikethrough(children)
            | BuildNode::Paragraph(children)
            | BuildNode::Heading { children, .. }
            | BuildNode::Link { children, .. }
            | BuildNode::WikiLink { children, .. } => out.push_str(&plain(children)),
            BuildNode::SoftBreak | BuildNode::HardBreak => out.push(' '),
            BuildNode::OpenBracket { prefix, .. } => {
                if let Some(p) = prefix {
                    out.push_str(p);
                }
                out.push('[');
            }
            _ => {}
        }
    }
    out
}

/// Parses one source. Total: never fails, never panics on any UTF-8 input.
#[must_use]
pub fn parse(source: &Source) -> Document {
    let text = &source.text;
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let mut b = Builder {
        stack: vec![(Frame::Other, Vec::new())],
        headings: Vec::new(),
        links: Vec::new(),
        diagnostics: Vec::new(),
        seen: BTreeMap::new(),
        used: std::collections::BTreeSet::new(),
        next_transclusion: 0,
        pending_refs: Vec::new(),
        reference_entries: Vec::new(),
        seen_entries: BTreeMap::new(),
        pending_citations: Vec::new(),
    };
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        let line = line_of(&starts, range.start);
        match event {
            Event::Start(tag) => b.start(tag, line),
            Event::End(end) => b.end(end),
            Event::Text(t) => b.text(&t, range.start, &starts),
            Event::Code(t) => {
                b.heading_text(&t);
                b.push_node(BuildNode::Code(t.to_string()));
            }
            Event::Html(t) | Event::InlineHtml(t) => {
                b.diagnostics
                    .push(Diagnostic::Unsupported { kind: "html", line });
                b.push_node(BuildNode::Html(t.to_string()));
            }
            Event::SoftBreak => {
                b.heading_text(" ");
                b.push_node(BuildNode::SoftBreak);
            }
            Event::HardBreak => b.push_node(BuildNode::HardBreak),
            Event::Rule => b.push_node(BuildNode::Rule),
            Event::TaskListMarker(done) => b.push_node(BuildNode::TaskMarker(done)),
            Event::InlineMath(t) | Event::DisplayMath(t) | Event::FootnoteReference(t) => {
                b.diagnostics.push(Diagnostic::Unsupported {
                    kind: "inline",
                    line,
                });
                b.push_node(BuildNode::Text(t.to_string()));
            }
        }
    }
    while b.stack.len() > 1 {
        b.end(TagEnd::Paragraph);
    }
    let mut body = b.stack.pop().map(|(_, c)| c).unwrap_or_default();
    register_entries(&mut body, &mut b);
    let (citations, citation_diagnostics, resolved_citations) =
        resolve_citations(&b.reference_entries, &b.pending_citations);
    let (section_refs, section_diagnostics, body) = resolve_section_refs(
        &b.headings,
        b.pending_refs,
        &b.pending_citations,
        &resolved_citations,
        body,
    );
    b.diagnostics.extend(citation_diagnostics);
    b.diagnostics.extend(section_diagnostics);
    let title = b
        .headings
        .iter()
        .find(|h| h.level == 1)
        .map_or_else(|| source.id.basename().to_owned(), |h| h.text.clone());
    Document {
        id: PageId::new(source.id.as_str()),
        source_digest: source.digest,
        title,
        headings: b.headings,
        links: b.links,
        body,
        diagnostics: b.diagnostics,
        section_refs,
        reference_entries: b.reference_entries,
        citations,
    }
}
