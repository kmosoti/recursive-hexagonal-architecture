use crate::Diagnostic;

/// Front matter metadata from the top of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub end_line: usize,
}

pub(crate) struct ExtractedFrontMatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub end_line: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub blanked_text: String,
}

fn strip_matching_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &value[1..bytes.len() - 1]
    } else {
        value
    }
}

fn is_blank_line(line: &str) -> bool {
    line.chars().all(|c| c == ' ' || c == '\t')
}

fn is_comment_line(line: &str) -> bool {
    line.trim_start_matches([' ', '\t']).starts_with('#')
}

fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
    {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b':' {
        let key = &line[..i];
        let raw_val = &line[i + 1..];
        let value = raw_val.trim_matches([' ', '\t']);
        Some((key, value))
    } else {
        None
    }
}

fn parse_list_item(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.is_empty() || bytes[0] != b' ' {
        return None;
    }
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'-' {
        let after_dash = &line[i + 1..];
        if after_dash.is_empty() {
            Some("")
        } else if let Some(item) = after_dash.strip_prefix(' ') {
            Some(item)
        } else {
            None
        }
    } else {
        None
    }
}

fn make_blanked_text(text: &str, end_line: usize) -> String {
    let mut out = String::new();
    let mut current_offset = 0;
    let mut lines_seen = 0;

    for (newline_pos, _) in text.match_indices('\n') {
        lines_seen += 1;
        if lines_seen <= end_line {
            if newline_pos > current_offset && text.as_bytes()[newline_pos - 1] == b'\r' {
                out.push_str("\r\n");
            } else {
                out.push('\n');
            }
            current_offset = newline_pos + 1;
            if lines_seen == end_line {
                break;
            }
        }
    }

    if lines_seen == end_line {
        out.push_str(&text[current_offset..]);
    }

    out
}

pub(crate) fn extract_front_matter(text: &str) -> Option<ExtractedFrontMatter> {
    if text.starts_with('\u{FEFF}') {
        return None;
    }

    let mut lines_iter = text.split('\n');
    let first_line = lines_iter.next()?;
    if first_line.trim_end_matches([' ', '\t', '\r']) != "---" {
        return None;
    }

    let mut closing_line_num = None;
    for (idx, line) in lines_iter.enumerate() {
        let line_num = idx + 2;
        let stripped = line.trim_end_matches([' ', '\t', '\r']);
        if stripped == "---" || stripped == "..." {
            closing_line_num = Some(line_num);
            break;
        }
    }

    let end_line = closing_line_num?;

    let mut title_captured = false;
    let mut title_val = None;
    let mut tags_captured = false;
    let mut tags_collector_active = false;
    let mut raw_tag_items: Vec<String> = Vec::new();
    let mut pending_key: Option<String> = None;
    let mut diagnostics = Vec::new();

    for (idx, raw_line) in text.split('\n').enumerate() {
        let line_num = idx + 1;
        if line_num == 1 {
            continue;
        }
        if line_num >= end_line {
            break;
        }

        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        if is_blank_line(line) {
            continue;
        }
        if is_comment_line(line) {
            continue;
        }

        if let Some((key, value)) = parse_key_value(line) {
            if key == "tags" && value.starts_with('[') && !value.ends_with(']') {
                diagnostics.push(Diagnostic::InvalidFrontMatter { line: line_num });
                continue;
            }

            pending_key = if value.is_empty() {
                Some(key.to_owned())
            } else {
                None
            };

            if key == "title" {
                if !title_captured {
                    title_captured = true;
                    let stripped = strip_matching_quotes(value);
                    if !stripped.is_empty() {
                        title_val = Some(stripped.to_owned());
                    }
                }
                continue;
            }

            if key == "tags" {
                if !tags_captured {
                    tags_captured = true;
                    if value.is_empty() {
                        tags_collector_active = true;
                    } else {
                        tags_collector_active = false;
                        if value.starts_with('[') && value.ends_with(']') {
                            let inner = &value[1..value.len() - 1];
                            if !inner.trim_matches([' ', '\t']).is_empty() {
                                for part in inner.split(',') {
                                    let trimmed = part.trim_matches([' ', '\t']);
                                    let unquoted = strip_matching_quotes(trimmed);
                                    raw_tag_items.push(unquoted.to_owned());
                                }
                            }
                        } else {
                            let unquoted = strip_matching_quotes(value);
                            raw_tag_items.push(unquoted.to_owned());
                        }
                    }
                } else {
                    tags_collector_active = false;
                }
                continue;
            }

            continue;
        }

        if let Some(item) = parse_list_item(line) {
            if pending_key.is_none() {
                diagnostics.push(Diagnostic::InvalidFrontMatter { line: line_num });
                continue;
            }
            if pending_key.as_deref() == Some("tags") && tags_collector_active {
                let trimmed = item.trim_matches([' ', '\t']);
                let unquoted = strip_matching_quotes(trimmed);
                raw_tag_items.push(unquoted.to_owned());
            }
            continue;
        }

        diagnostics.push(Diagnostic::InvalidFrontMatter { line: line_num });
    }

    let mut tags = Vec::new();
    let mut seen_casefold = std::collections::BTreeSet::new();
    for raw in raw_tag_items {
        let tag = raw.trim_matches([' ', '\t']);
        if tag.is_empty() {
            continue;
        }
        let key = tag.to_ascii_lowercase();
        if seen_casefold.insert(key) {
            tags.push(tag.to_owned());
        }
    }

    let blanked_text = make_blanked_text(text, end_line);

    Some(ExtractedFrontMatter {
        title: title_val,
        tags,
        end_line,
        diagnostics,
        blanked_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_line_endings_everywhere() {
        let text = "---\r\ntitle: CRLF Title\r\ntags:\r\n - rust\r\n---\r\n# Heading\r\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.title.as_deref(), Some("CRLF Title"));
        assert_eq!(extracted.tags, vec!["rust"]);
        assert_eq!(extracted.end_line, 5);
        assert!(extracted.diagnostics.is_empty());
        assert_eq!(extracted.blanked_text, "\r\n\r\n\r\n\r\n\r\n# Heading\r\n");
    }

    #[test]
    fn delimiter_trailing_spaces_and_tabs_crlf() {
        let text = "---\t \r\ntitle: Mixed\r\n... \t\r\nBody\r\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.title.as_deref(), Some("Mixed"));
        assert_eq!(extracted.end_line, 3);
        assert!(extracted.diagnostics.is_empty());
    }

    #[test]
    fn unclosed_flow_list_followed_by_valid_tags() {
        let text = "---\ntags: [unclosed\ntags: [valid]\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.tags, vec!["valid"]);
        assert_eq!(
            extracted.diagnostics,
            vec![Diagnostic::InvalidFrontMatter { line: 2 }]
        );
    }

    #[test]
    fn space_before_colon_is_invalid() {
        let text = "---\ntitle : Invalid\ntags: [a]\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.title, None);
        assert_eq!(extracted.tags, vec!["a"]);
        assert_eq!(
            extracted.diagnostics,
            vec![Diagnostic::InvalidFrontMatter { line: 2 }]
        );
    }

    #[test]
    fn empty_list_items_dropped_without_diagnostic() {
        let text = "---\ntags:\n - \n -\n - valid\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.tags, vec!["valid"]);
        assert!(extracted.diagnostics.is_empty());
    }

    #[test]
    fn tab_indented_and_unindented_list_items() {
        let text = "---\ntags:\n\t- tab\n- unindented\n - valid\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.tags, vec!["valid"]);
        assert_eq!(
            extracted.diagnostics,
            vec![
                Diagnostic::InvalidFrontMatter { line: 3 },
                Diagnostic::InvalidFrontMatter { line: 4 },
            ]
        );
    }

    #[test]
    fn quoted_comma_in_flow_list() {
        let text = "---\ntags: [a, \"b, c\"]\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.tags, vec!["a", "\"b", "c\""]);
        assert!(extracted.diagnostics.is_empty());
    }

    #[test]
    fn quote_removal_on_scalar_block_and_flow() {
        let text = "---\ntitle: 'Quoted Title'\ntags: \"scalar\"\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.title.as_deref(), Some("Quoted Title"));
        assert_eq!(extracted.tags, vec!["scalar"]);

        let text2 = "---\ntags:\n - 'single'\n - \"double\"\n---\n";
        let extracted2 = extract_front_matter(text2).expect("recognized");
        assert_eq!(extracted2.tags, vec!["single", "double"]);
    }

    #[test]
    fn case_insensitive_dedup_first_spelling() {
        let text = "---\ntags: [Alpha, alpha, ALPHA, beta, BETA]\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.tags, vec!["Alpha", "beta"]);
    }

    #[test]
    fn delimiter_without_trailing_newline_at_eof() {
        let text = "---\ntitle: EOF\n---";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.title.as_deref(), Some("EOF"));
        assert_eq!(extracted.end_line, 3);
        assert_eq!(extracted.blanked_text, "\n\n");
    }

    #[test]
    fn non_ascii_tags_preserved_and_deduped() {
        let text = "---\ntags: [café, café, résumé]\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.tags, vec!["café", "résumé"]);
    }

    #[test]
    fn non_ascii_keys_are_invalid() {
        let text = "---\nключ: val\ntitle: Valid\n---\n";
        let extracted = extract_front_matter(text).expect("recognized");
        assert_eq!(extracted.title.as_deref(), Some("Valid"));
        assert_eq!(
            extracted.diagnostics,
            vec![Diagnostic::InvalidFrontMatter { line: 2 }]
        );
    }
}
