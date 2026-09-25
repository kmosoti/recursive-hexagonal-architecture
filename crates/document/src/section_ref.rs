use std::collections::BTreeMap;

use crate::{Diagnostic, Heading, Node, citation::PendingCitation, parse::BuildNode};

/// A section reference (§n.n).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionRef {
    pub text: String,
    pub number: String,
    pub target: Option<String>,
    pub line: usize,
}

pub struct PendingRef {
    pub text: String,
    pub number: String,
    pub line: usize,
}

pub enum ScannedPiece {
    Text(String),
    Ref {
        ref_text: String,
        number: String,
        line: usize,
    },
}

/// Extracts the section number of a heading from its text, per contract section 1.
#[must_use]
pub fn heading_section_number(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    // Form 1: N(.N)*, optionally followed by one '.'
    if bytes[0].is_ascii_digit() {
        let mut i = 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        while i < bytes.len()
            && bytes[i] == b'.'
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_digit()
        {
            i += 1; // skip '.'
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        let number = &text[..i];
        if i == text.len() {
            return Some(number.to_owned());
        }
        if bytes[i] == b'.' {
            let after_dot = i + 1;
            if after_dot == text.len() {
                return Some(number.to_owned());
            }
            if text[after_dot..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                return Some(number.to_owned());
            }
            return None;
        }
        if text[i..].chars().next().is_some_and(char::is_whitespace) {
            return Some(number.to_owned());
        }
        return None;
    }

    // Form 2: Appendix L(.N)*, optionally followed by one '.'
    if let Some(rem) = text.strip_prefix("Appendix") {
        let ws_len = rem
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
        if ws_len == 0 {
            return None;
        }
        let rem = &rem[ws_len..];
        let rem_bytes = rem.as_bytes();
        if rem_bytes.is_empty() || !rem_bytes[0].is_ascii_uppercase() {
            return None;
        }
        let mut i = 1;
        while i < rem_bytes.len()
            && rem_bytes[i] == b'.'
            && i + 1 < rem_bytes.len()
            && rem_bytes[i + 1].is_ascii_digit()
        {
            i += 1; // skip '.'
            while i < rem_bytes.len() && rem_bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        let number = &rem[..i];
        if i == rem.len() {
            return Some(number.to_owned());
        }
        if rem_bytes[i] == b'.' {
            let after_dot = i + 1;
            if after_dot == rem.len() {
                return Some(number.to_owned());
            }
            if rem[after_dot..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                return Some(number.to_owned());
            }
            return None;
        }
        if rem[i..].chars().next().is_some_and(char::is_whitespace) {
            return Some(number.to_owned());
        }
        return None;
    }

    None
}

/// Matches a section number: N(.N)* or L(.N)*, where L counts only when the character
/// after it is not an ASCII letter.
fn match_number(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    if bytes[0].is_ascii_digit() {
        let mut i = 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        while i < bytes.len()
            && bytes[i] == b'.'
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_digit()
        {
            i += 1; // skip '.'
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        Some(&s[..i])
    } else if bytes[0].is_ascii_uppercase() {
        if bytes.len() > 1 && bytes[1].is_ascii_alphabetic() {
            return None;
        }
        let mut i = 1;
        while i < bytes.len()
            && bytes[i] == b'.'
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_digit()
        {
            i += 1; // skip '.'
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        Some(&s[..i])
    } else {
        None
    }
}

/// Matches a range continuation directly after a number:
/// optional whitespace (U+0020 only), a dash ('–' or '-'), optional whitespace,
/// an optional single '§' (rejecting '§§'), and a number.
///
/// Returns (`range_separator`, `end_ref_text`, `end_number`, `total_consumed_bytes`).
fn match_range_continuation(s: &str) -> Option<(&str, &str, &str, usize)> {
    let bytes = s.as_bytes();
    let mut k = 0;
    while k < bytes.len() && bytes[k] == b' ' {
        k += 1;
    }
    if s[k..].starts_with('–') {
        k += '–'.len_utf8();
    } else if s[k..].starts_with('-') {
        k += 1;
    } else {
        return None;
    }
    while k < bytes.len() && bytes[k] == b' ' {
        k += 1;
    }
    if s[k..].starts_with("§§") {
        return None;
    }
    let sign_start = k;
    if s[k..].starts_with('§') {
        k += '§'.len_utf8();
    }
    let num = match_number(&s[k..])?;
    let end_ref_text = &s[sign_start..k + num.len()];
    let end_number = num;
    let range_sep = &s[..sign_start];
    let total_consumed = k + num.len();
    Some((range_sep, end_ref_text, end_number, total_consumed))
}

#[must_use]
pub fn scan_references(text: &str, base_offset: usize, starts: &[usize]) -> Vec<ScannedPiece> {
    let mut pieces = Vec::new();
    let mut last_flush = 0;
    let mut search_pos = 0;
    let bytes = text.as_bytes();

    while search_pos < text.len() {
        let Some(rel_i) = text[search_pos..].find('§') else {
            break;
        };
        let i = search_pos + rel_i;

        let sign_len = if text[i..].starts_with("§§") {
            "§§".len()
        } else {
            "§".len()
        };
        let mut cur = i + sign_len;
        if cur < text.len() && bytes[cur] == b' ' {
            cur += 1;
        }

        if let Some(number) = match_number(&text[cur..]) {
            let ref_start = i;
            let ref_end = cur + number.len();
            let ref_text = text[ref_start..ref_end].to_owned();
            let number_str = number.to_owned();
            let ref_line = starts.partition_point(|&s| s <= base_offset + ref_start);
            let mut pos = ref_end;

            if let Some((range_sep, end_text, end_num, total_consumed)) =
                match_range_continuation(&text[pos..])
            {
                let sep_str = range_sep.to_owned();
                let end_text_str = end_text.to_owned();
                let end_number_str = end_num.to_owned();
                let end_ref_start = pos + range_sep.len();
                let end_ref_line = starts.partition_point(|&s| s <= base_offset + end_ref_start);
                pos += total_consumed;

                if ref_start > last_flush {
                    pieces.push(ScannedPiece::Text(text[last_flush..ref_start].to_owned()));
                }
                pieces.push(ScannedPiece::Ref {
                    ref_text,
                    number: number_str,
                    line: ref_line,
                });
                pieces.push(ScannedPiece::Text(sep_str));
                pieces.push(ScannedPiece::Ref {
                    ref_text: end_text_str,
                    number: end_number_str,
                    line: end_ref_line,
                });
                last_flush = pos;
                search_pos = pos;
            } else {
                if ref_start > last_flush {
                    pieces.push(ScannedPiece::Text(text[last_flush..ref_start].to_owned()));
                }
                pieces.push(ScannedPiece::Ref {
                    ref_text,
                    number: number_str,
                    line: ref_line,
                });
                last_flush = pos;
                search_pos = pos;
            }
        } else {
            search_pos = i + '§'.len_utf8();
        }
    }

    if last_flush < text.len() {
        pieces.push(ScannedPiece::Text(text[last_flush..].to_owned()));
    }

    pieces
}

#[must_use]
pub fn resolve_section_refs(
    headings: &[Heading],
    pending_refs: Vec<PendingRef>,
    pending_citations: &[PendingCitation],
    resolved_citations: &[Option<String>],
    body: Vec<BuildNode>,
) -> (Vec<SectionRef>, Vec<Diagnostic>, Vec<Node>) {
    let mut heading_numbers = BTreeMap::new();
    for heading in headings {
        if let Some(number) = heading_section_number(&heading.text) {
            heading_numbers
                .entry(number)
                .or_insert_with(|| heading.slug.clone());
        }
    }

    let has_numbered_headings = !heading_numbers.is_empty();
    let mut section_refs = Vec::with_capacity(pending_refs.len());
    let mut diagnostics = Vec::new();

    for pending in pending_refs {
        let target = heading_numbers.get(&pending.number).cloned();
        if target.is_none() && has_numbered_headings {
            diagnostics.push(Diagnostic::UnresolvedSectionRef {
                number: pending.number.clone(),
                line: pending.line,
            });
        }
        section_refs.push(SectionRef {
            text: pending.text,
            number: pending.number,
            target,
            line: pending.line,
        });
    }

    let new_body = resolve_and_flatten(body, &section_refs, pending_citations, resolved_citations);

    (section_refs, diagnostics, new_body)
}

fn resolve_and_flatten(
    nodes: Vec<BuildNode>,
    section_refs: &[SectionRef],
    pending_citations: &[PendingCitation],
    resolved_citations: &[Option<String>],
) -> Vec<Node> {
    let mut out = Vec::with_capacity(nodes.len());
    let mut just_flattened_ref = false;

    for node in nodes {
        match node {
            BuildNode::Heading {
                level,
                slug,
                children,
            } => {
                just_flattened_ref = false;
                out.push(Node::Heading {
                    level,
                    slug,
                    children: resolve_and_flatten(
                        children,
                        section_refs,
                        pending_citations,
                        resolved_citations,
                    ),
                });
            }
            BuildNode::Paragraph(children) => {
                just_flattened_ref = false;
                out.push(Node::Paragraph(resolve_and_flatten(
                    children,
                    section_refs,
                    pending_citations,
                    resolved_citations,
                )));
            }
            BuildNode::Emphasis(children) => {
                just_flattened_ref = false;
                out.push(Node::Emphasis(resolve_and_flatten(
                    children,
                    section_refs,
                    pending_citations,
                    resolved_citations,
                )));
            }
            BuildNode::Strong(children) => {
                just_flattened_ref = false;
                out.push(Node::Strong(resolve_and_flatten(
                    children,
                    section_refs,
                    pending_citations,
                    resolved_citations,
                )));
            }
            BuildNode::Strikethrough(children) => {
                just_flattened_ref = false;
                out.push(Node::Strikethrough(resolve_and_flatten(
                    children,
                    section_refs,
                    pending_citations,
                    resolved_citations,
                )));
            }
            BuildNode::Link { href, children } => {
                just_flattened_ref = false;
                out.push(Node::Link {
                    href,
                    children: resolve_and_flatten(
                        children,
                        section_refs,
                        pending_citations,
                        resolved_citations,
                    ),
                });
            }
            BuildNode::WikiLink { index, children } => {
                just_flattened_ref = false;
                out.push(Node::WikiLink {
                    index,
                    children: resolve_and_flatten(
                        children,
                        section_refs,
                        pending_citations,
                        resolved_citations,
                    ),
                });
            }
            BuildNode::BlockQuote { kind, children } => {
                just_flattened_ref = false;
                out.push(Node::BlockQuote {
                    kind,
                    children: resolve_and_flatten(
                        children,
                        section_refs,
                        pending_citations,
                        resolved_citations,
                    ),
                });
            }
            BuildNode::List { start, items } => {
                just_flattened_ref = false;
                out.push(Node::List {
                    start,
                    items: items
                        .into_iter()
                        .map(|item| {
                            resolve_and_flatten(
                                item,
                                section_refs,
                                pending_citations,
                                resolved_citations,
                            )
                        })
                        .collect(),
                });
            }
            BuildNode::Table { align, head, rows } => {
                just_flattened_ref = false;
                out.push(Node::Table {
                    align,
                    head: head
                        .into_iter()
                        .map(|cell| {
                            resolve_and_flatten(
                                cell,
                                section_refs,
                                pending_citations,
                                resolved_citations,
                            )
                        })
                        .collect(),
                    rows: rows
                        .into_iter()
                        .map(|row| {
                            row.into_iter()
                                .map(|cell| {
                                    resolve_and_flatten(
                                        cell,
                                        section_refs,
                                        pending_citations,
                                        resolved_citations,
                                    )
                                })
                                .collect()
                        })
                        .collect(),
                });
            }
            BuildNode::Transclusion(transclusion) => {
                just_flattened_ref = false;
                out.push(Node::Transclusion(transclusion));
            }
            BuildNode::Image { src, alt, .. } => {
                just_flattened_ref = false;
                out.push(Node::Image { src, alt });
            }
            BuildNode::Text(text) => {
                if just_flattened_ref {
                    if let Some(Node::Text(last)) = out.last_mut() {
                        last.push_str(&text);
                    } else {
                        out.push(Node::Text(text));
                    }
                    just_flattened_ref = false;
                } else {
                    out.push(Node::Text(text));
                }
            }
            BuildNode::Code(text) => {
                just_flattened_ref = false;
                out.push(Node::Code(text));
            }
            BuildNode::CodeBlock { lang, text } => {
                just_flattened_ref = false;
                out.push(Node::CodeBlock { lang, text });
            }
            BuildNode::Rule => {
                just_flattened_ref = false;
                out.push(Node::Rule);
            }
            BuildNode::SoftBreak => {
                just_flattened_ref = false;
                out.push(Node::SoftBreak);
            }
            BuildNode::HardBreak => {
                just_flattened_ref = false;
                out.push(Node::HardBreak);
            }
            BuildNode::Html(text) => {
                just_flattened_ref = false;
                out.push(Node::Html(text));
            }
            BuildNode::TaskMarker(checked) => {
                just_flattened_ref = false;
                out.push(Node::TaskMarker(checked));
            }
            BuildNode::Anchor { id } => {
                just_flattened_ref = false;
                out.push(Node::Anchor { id });
            }
            BuildNode::OpenBracket { prefix, .. } => {
                // Rejoin the prefix that was split from the same text event.
                let combined = match prefix {
                    Some(p) => {
                        let mut s = p;
                        s.push('[');
                        s
                    }
                    None => "[".to_owned(),
                };
                if just_flattened_ref {
                    if let Some(Node::Text(last)) = out.last_mut() {
                        last.push_str(&combined);
                    } else {
                        out.push(Node::Text(combined));
                    }
                    just_flattened_ref = false;
                } else {
                    out.push(Node::Text(combined));
                }
            }
            BuildNode::Citation(cit_idx) => {
                let cit = &pending_citations[cit_idx];
                if cit.is_entry_label {
                    let formatted = format!("[{}]", cit.label);
                    if just_flattened_ref {
                        if let Some(Node::Text(last)) = out.last_mut() {
                            last.push_str(&formatted);
                        } else {
                            out.push(Node::Text(formatted));
                        }
                        just_flattened_ref = false;
                    } else {
                        out.push(Node::Text(formatted));
                    }
                } else if let Some(target) = &resolved_citations[cit_idx] {
                    just_flattened_ref = false;
                    out.push(Node::Link {
                        href: format!("#{target}"),
                        children: vec![Node::Text(format!("[{}]", cit.label))],
                    });
                } else {
                    let formatted = format!("[{}]", cit.label);
                    if just_flattened_ref {
                        if let Some(Node::Text(last)) = out.last_mut() {
                            last.push_str(&formatted);
                        } else {
                            out.push(Node::Text(formatted));
                        }
                        just_flattened_ref = false;
                    } else {
                        out.push(Node::Text(formatted));
                    }
                }
            }
            BuildNode::SectionRef(ref_idx) => {
                let sref = &section_refs[ref_idx];
                if let Some(slug) = &sref.target {
                    out.push(Node::Link {
                        href: format!("#{slug}"),
                        children: vec![Node::Text(sref.text.clone())],
                    });
                    just_flattened_ref = false;
                } else {
                    if let Some(Node::Text(last)) = out.last_mut() {
                        last.push_str(&sref.text);
                    } else {
                        out.push(Node::Text(sref.text.clone()));
                    }
                    just_flattened_ref = true;
                }
            }
        }
    }
    out
}
