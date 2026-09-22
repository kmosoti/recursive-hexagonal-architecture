use std::collections::BTreeMap;

use library::{PageId, Source};
use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag,
    TagEnd,
};

use crate::{Align, CalloutKind, Diagnostic, Document, Heading, Link, Node, slugify};

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
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<Node>>,
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
        head: Vec<Vec<Node>>,
        rows: Vec<Vec<Vec<Node>>>,
        in_head: bool,
    },
    Row {
        cells: Vec<Vec<Node>>,
    },
    Cell,
    Other,
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
    stack: Vec<(Frame, Vec<Node>)>,
    headings: Vec<Heading>,
    links: Vec<Link>,
    diagnostics: Vec<Diagnostic>,
    seen: BTreeMap<String, (usize, usize)>,
    /// Every final slug allocated so far on the page.
    used: std::collections::BTreeSet<String>,
}

impl Builder {
    fn push_node(&mut self, node: Node) {
        if let Some((_, children)) = self.stack.last_mut() {
            children.push(node);
        }
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
            Tag::Image { dest_url, .. } => Frame::Image {
                src: dest_url.to_string(),
                alt: String::new(),
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
                Some(Node::Heading {
                    level,
                    slug,
                    children,
                })
            }
            Frame::Paragraph => Some(Node::Paragraph(children)),
            Frame::Emphasis => Some(Node::Emphasis(children)),
            Frame::Strong => Some(Node::Strong(children)),
            Frame::Strike => Some(Node::Strikethrough(children)),
            Frame::Link { href } => Some(Node::Link { href, children }),
            Frame::WikiLink { index } => {
                if let Some(link) = self.links.get_mut(index)
                    && link.alias.is_some()
                {
                    link.alias = Some(plain(&children));
                }
                Some(Node::WikiLink { index, children })
            }
            Frame::Image { src, alt } => Some(Node::Image { src, alt }),
            Frame::List { start, items } => Some(Node::List { start, items }),
            Frame::Item => {
                if let Some((Frame::List { items, .. }, _)) = self.stack.last_mut() {
                    items.push(children);
                }
                None
            }
            Frame::Quote { kind } => Some(Node::BlockQuote { kind, children }),
            Frame::CodeBlock { lang, text } => Some(Node::CodeBlock { lang, text }),
            Frame::Table {
                align, head, rows, ..
            } => Some(Node::Table { align, head, rows }),
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
            Frame::Other => Some(Node::Paragraph(children)),
        };
        if let Some(node) = node {
            self.push_node(node);
        }
    }

    fn text(&mut self, text: &str) {
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
        self.push_node(Node::Text(text.to_owned()));
    }
}

/// The plain text of a node list.
fn plain(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(t) | Node::Code(t) => out.push_str(t),
            Node::Emphasis(c) | Node::Strong(c) | Node::Strikethrough(c) | Node::Paragraph(c) => {
                out.push_str(&plain(c))
            }
            Node::Link { children, .. }
            | Node::WikiLink { children, .. }
            | Node::Heading { children, .. } => out.push_str(&plain(children)),
            Node::SoftBreak | Node::HardBreak => out.push(' '),
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
    };
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        let line = line_of(&starts, range.start);
        match event {
            Event::Start(tag) => b.start(tag, line),
            Event::End(end) => b.end(end),
            Event::Text(t) => b.text(&t),
            Event::Code(t) => {
                b.heading_text(&t);
                b.push_node(Node::Code(t.to_string()));
            }
            Event::Html(t) | Event::InlineHtml(t) => {
                b.diagnostics
                    .push(Diagnostic::Unsupported { kind: "html", line });
                b.push_node(Node::Html(t.to_string()));
            }
            Event::SoftBreak => {
                b.heading_text(" ");
                b.push_node(Node::SoftBreak);
            }
            Event::HardBreak => b.push_node(Node::HardBreak),
            Event::Rule => b.push_node(Node::Rule),
            Event::TaskListMarker(done) => b.push_node(Node::TaskMarker(done)),
            Event::InlineMath(t) | Event::DisplayMath(t) | Event::FootnoteReference(t) => {
                b.diagnostics.push(Diagnostic::Unsupported {
                    kind: "inline",
                    line,
                });
                b.push_node(Node::Text(t.to_string()));
            }
        }
    }
    while b.stack.len() > 1 {
        b.end(TagEnd::Paragraph);
    }
    let body = b.stack.pop().map(|(_, c)| c).unwrap_or_default();
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
    }
}
