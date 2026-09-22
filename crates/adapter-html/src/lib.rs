//! HTML rendering: the presentational part of rhawiki (plan §3). Escaping
//! happens in one function, [`escape`], which every text path goes through.

use std::fmt::Write as _;

use site::assembly::{LinkTarget, PageModel};
use site::{Align, CalloutKind, Node, PageId, PageRenderer, RelPath, Rendered};

/// Escapes text for element content and double-quoted attributes.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Renders page models to `<id>.html` with a shared stylesheet.
#[derive(Debug, Clone, Copy, Default)]
pub struct HtmlRenderer;

const STYLE: &str = "body{font-family:system-ui,sans-serif;max-width:52rem;margin:2rem auto;padding:0 1rem;line-height:1.5}\nnav.toc{font-size:.9rem}\n.broken{color:#b00020;text-decoration:underline dotted}\npre{overflow-x:auto;background:#f5f5f5;padding:.5rem}\nblockquote{border-left:3px solid #ccc;margin-left:0;padding-left:1rem}\ntable{border-collapse:collapse}td,th{border:1px solid #ccc;padding:.2rem .4rem}\nfooter{margin-top:3rem;font-size:.8rem;color:#666}\n";

/// Percent-encodes everything outside RFC 3986's unreserved set, keeping
/// `/` when `keep_slash`, so a page id like `Budget?2026` stays a path and
/// never becomes a query (review finding 4).
#[must_use]
pub fn encode(text: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'.' | b'_' | b'~')
            || (keep_slash && b == b'/')
        {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

fn relative(from: &PageId, to: &PageId) -> String {
    let depth = from.as_str().matches('/').count();
    format!("{}{}.html", "../".repeat(depth), encode(to.as_str(), true))
}

/// Whether a reference starts with a URI scheme (RFC 3986 §3.1): a letter,
/// then letters, digits, `+`, `-` or `.`, then `:`, all before any `/`, `?`
/// or `#`. So `mailto:x` has one and `./a:b.md` does not.
fn has_scheme(href: &str) -> bool {
    let Some((head, _)) = href.split_once(':') else {
        return false;
    };
    let mut chars = head.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// An ordinary link: a relative reference whose PATH ends in `.md` points at
/// the `.html` page the build produces. The query and fragment are kept byte
/// for byte, so `download?file=manual.md` is untouched (its path is
/// `download`) and `x.md?print=1#sec` becomes `x.html?print=1#sec`. Absolute
/// paths, scheme URLs and fragment-only links are left as written (review
/// findings 5 and round 2).
#[must_use]
pub fn rewrite_href(href: &str) -> String {
    if href.starts_with('/') || href.starts_with('#') || has_scheme(href) {
        return href.to_owned();
    }
    let (before_fragment, fragment) = href
        .split_once('#')
        .map_or((href, None), |(p, f)| (p, Some(f)));
    let (path, query) = before_fragment
        .split_once('?')
        .map_or((before_fragment, None), |(p, q)| (p, Some(q)));
    match path.strip_suffix(".md") {
        Some(stem) if !stem.is_empty() && !stem.ends_with('/') => {
            let mut out = format!("{stem}.html");
            if let Some(q) = query {
                out.push('?');
                out.push_str(q);
            }
            if let Some(f) = fragment {
                out.push('#');
                out.push_str(f);
            }
            out
        }
        _ => href.to_owned(),
    }
}

struct Ctx<'a> {
    page: &'a PageModel,
}

impl Ctx<'_> {
    fn nodes(&self, out: &mut String, nodes: &[Node]) {
        for n in nodes {
            self.node(out, n);
        }
    }

    fn node(&self, out: &mut String, node: &Node) {
        match node {
            Node::Heading {
                level,
                slug,
                children,
            } => {
                let _ = write!(out, "\n<h{level} id=\"{}\">", escape(slug));
                self.nodes(out, children);
                let _ = writeln!(out, "</h{level}>");
            }
            Node::Paragraph(c) => {
                out.push_str("<p>");
                self.nodes(out, c);
                out.push_str("</p>\n");
            }
            Node::Text(t) | Node::Html(t) => out.push_str(&escape(t)),
            Node::Code(t) => {
                let _ = write!(out, "<code>{}</code>", escape(t));
            }
            Node::CodeBlock { lang, text } => match lang.as_deref() {
                Some("mermaid") => {
                    let _ = writeln!(out, "<pre class=\"mermaid\">{}</pre>", escape(text));
                }
                Some(l) => {
                    let _ = writeln!(
                        out,
                        "<pre><code class=\"language-{}\">{}</code></pre>",
                        escape(l),
                        escape(text)
                    );
                }
                None => {
                    let _ = writeln!(out, "<pre><code>{}</code></pre>", escape(text));
                }
            },
            Node::Emphasis(c) => self.wrap(out, "em", c),
            Node::Strong(c) => self.wrap(out, "strong", c),
            Node::Strikethrough(c) => self.wrap(out, "del", c),
            Node::Link { href, children } => {
                let _ = write!(out, "<a href=\"{}\">", escape(&rewrite_href(href)));
                self.nodes(out, children);
                out.push_str("</a>");
            }
            Node::WikiLink { index, children } => match self.page.links.get(*index) {
                Some(LinkTarget::Page { id, anchor }) => {
                    let href = if id == &self.page.id {
                        String::new()
                    } else {
                        relative(&self.page.id, id)
                    };
                    let frag = anchor
                        .as_ref()
                        .map(|a| format!("#{}", encode(a, false)))
                        .unwrap_or_default();
                    let _ = write!(
                        out,
                        "<a class=\"wikilink\" href=\"{}{}\">",
                        escape(&href),
                        escape(&frag)
                    );
                    self.nodes(out, children);
                    out.push_str("</a>");
                }
                _ => {
                    out.push_str("<span class=\"broken\">");
                    self.nodes(out, children);
                    out.push_str("</span>");
                }
            },
            Node::Image { src, alt } => {
                let _ = write!(out, "<img src=\"{}\" alt=\"{}\">", escape(src), escape(alt));
            }
            Node::List { start, items } => {
                let tag = if start.is_some() { "ol" } else { "ul" };
                match start {
                    Some(s) if *s != 1 => {
                        let _ = writeln!(out, "<ol start=\"{s}\">");
                    }
                    _ => {
                        let _ = writeln!(out, "<{tag}>");
                    }
                }
                for item in items {
                    out.push_str("<li>");
                    self.nodes(out, item);
                    out.push_str("</li>\n");
                }
                let _ = writeln!(out, "</{tag}>");
            }
            Node::BlockQuote { kind, children } => {
                let class = kind.map(|k| match k {
                    CalloutKind::Note => "note",
                    CalloutKind::Tip => "tip",
                    CalloutKind::Important => "important",
                    CalloutKind::Warning => "warning",
                    CalloutKind::Caution => "caution",
                });
                match class {
                    Some(c) => {
                        let _ = writeln!(out, "<blockquote class=\"callout {c}\">");
                    }
                    None => out.push_str("<blockquote>\n"),
                }
                self.nodes(out, children);
                out.push_str("</blockquote>\n");
            }
            Node::Table { align, head, rows } => {
                out.push_str("<table>\n<thead><tr>");
                for (i, cell) in head.iter().enumerate() {
                    self.cell(out, "th", align.get(i).copied(), cell);
                }
                out.push_str("</tr></thead>\n<tbody>\n");
                for row in rows {
                    out.push_str("<tr>");
                    for (i, cell) in row.iter().enumerate() {
                        self.cell(out, "td", align.get(i).copied(), cell);
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n</table>\n");
            }
            Node::Rule => out.push_str("<hr>\n"),
            Node::SoftBreak => out.push('\n'),
            Node::HardBreak => out.push_str("<br>\n"),
            Node::TaskMarker(done) => out.push_str(if *done {
                "<input type=\"checkbox\" checked disabled> "
            } else {
                "<input type=\"checkbox\" disabled> "
            }),
        }
    }

    fn wrap(&self, out: &mut String, tag: &str, children: &[Node]) {
        let _ = write!(out, "<{tag}>");
        self.nodes(out, children);
        let _ = write!(out, "</{tag}>");
    }

    fn cell(&self, out: &mut String, tag: &str, align: Option<Align>, cell: &[Node]) {
        let style = match align {
            Some(Align::Left) => " style=\"text-align:left\"",
            Some(Align::Center) => " style=\"text-align:center\"",
            Some(Align::Right) => " style=\"text-align:right\"",
            _ => "",
        };
        let _ = write!(out, "<{tag}{style}>");
        self.nodes(out, cell);
        let _ = write!(out, "</{tag}>");
    }
}

impl PageRenderer for HtmlRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        let depth = page.id.as_str().matches('/').count();
        let mut out = String::new();
        let _ = write!(
            out,
            "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<link rel=\"stylesheet\" href=\"{}assets/style.css\">\n</head>\n<body>\n",
            escape(&page.title),
            "../".repeat(depth)
        );
        if !page.breadcrumbs.is_empty() {
            let _ = writeln!(
                out,
                "<nav class=\"breadcrumbs\">{}</nav>",
                page.breadcrumbs
                    .iter()
                    .map(|b| escape(b))
                    .collect::<Vec<_>>()
                    .join(" / ")
            );
        }
        if !page.toc.is_empty() {
            out.push_str("<nav class=\"toc\"><ul>\n");
            for e in &page.toc {
                let _ = writeln!(
                    out,
                    "<li class=\"l{}\"><a href=\"#{}\">{}</a></li>",
                    e.level,
                    escape(&e.anchor),
                    escape(&e.text)
                );
            }
            out.push_str("</ul></nav>\n");
        }
        out.push_str("<main>\n");
        Ctx { page }.nodes(&mut out, &page.body);
        out.push_str("</main>\n");
        if !page.backlinks.is_empty() {
            out.push_str(
                "<aside class=\"backlinks\"><h2 class=\"backlinks-title\">Linked from</h2><ul>\n",
            );
            for (id, title) in &page.backlinks {
                let _ = writeln!(
                    out,
                    "<li><a href=\"{}\">{}</a></li>",
                    escape(&relative(&page.id, id)),
                    escape(title)
                );
            }
            out.push_str("</ul></aside>\n");
        }
        if let Some(t) = &page.built_at {
            let _ = writeln!(out, "<footer>Built at {}</footer>", escape(t));
        }
        out.push_str("</body>\n</html>\n");
        let name = format!("{}.html", page.id.as_str());
        #[allow(clippy::expect_used)]
        let path = RelPath::new(&name).expect("a page id plus .html is a relative path");
        Rendered {
            path,
            bytes: out.into_bytes(),
        }
    }

    fn assets(&self) -> Vec<Rendered> {
        #[allow(clippy::expect_used)]
        let path = RelPath::new("assets/style.css").expect("static path");
        vec![Rendered {
            path,
            bytes: STYLE.as_bytes().to_vec(),
        }]
    }
}
