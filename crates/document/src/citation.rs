//! Citation and reference entry parsing and resolution (CHG-011).

use std::collections::BTreeMap;

use crate::Diagnostic;

/// A reference entry under `# References`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEntry {
    pub label: String,
    pub anchor: String,
    pub line: usize,
}

/// A citation ([Rn]) in text content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub label: String,
    pub target: Option<String>,
    pub line: usize,
}

/// A citation token recognized during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCitation {
    pub label: String,
    pub line: usize,
    pub is_entry_label: bool,
}

/// Matches a citation label: `R\d+[a-z]?`.
#[must_use]
pub fn parse_citation_label(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'R' || !bytes[1].is_ascii_digit() {
        return None;
    }
    let mut i = 2;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i].is_ascii_lowercase() {
        i += 1;
    }
    if i == bytes.len() { Some(s) } else { None }
}

/// Resolves citations against reference entries on the page.
///
/// Returns (citations, diagnostics, `resolved_targets`).
#[must_use]
pub fn resolve_citations(
    reference_entries: &[ReferenceEntry],
    pending_citations: &[PendingCitation],
) -> (Vec<Citation>, Vec<Diagnostic>, Vec<Option<String>>) {
    let mut entry_by_label = BTreeMap::new();
    for entry in reference_entries {
        entry_by_label.insert(entry.label.as_str(), &entry.anchor);
    }
    let has_entries = !reference_entries.is_empty();

    let mut citations = Vec::new();
    let mut diagnostics = Vec::new();
    let mut resolved_targets = Vec::with_capacity(pending_citations.len());

    for pending in pending_citations {
        if pending.is_entry_label {
            resolved_targets.push(None);
            continue;
        }

        let target = entry_by_label.get(pending.label.as_str()).copied().cloned();
        if target.is_none() && has_entries {
            diagnostics.push(Diagnostic::UnresolvedCitation {
                label: pending.label.clone(),
                line: pending.line,
            });
        }
        citations.push(Citation {
            label: pending.label.clone(),
            target: target.clone(),
            line: pending.line,
        });
        resolved_targets.push(target);
    }

    (citations, diagnostics, resolved_targets)
}
