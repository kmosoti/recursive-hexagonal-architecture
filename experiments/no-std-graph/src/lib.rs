//! `graph`: resolve every wikilink or witness why not (plan §3.2).
//!
//! Resolution (the registered contract): an exact page id wins; otherwise the
//! unique page whose basename equals the target case-insensitively;
//! otherwise the link is broken, or ambiguous when several match. An anchor
//! must equal one of the target page's final slugs.
#![no_std]
#![forbid(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::disallowed_macros
)]
extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::{string::String, vec::Vec};

use document::Document;
use library::PageId;

#[cfg(feature = "negative-std-probe")]
fn negative_std_probe() {
    let _: std::collections::BTreeMap<(), ()> = std::collections::BTreeMap::new();
}

/// A link that resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub from: PageId,
    /// Index into the source document's `links`.
    pub index: usize,
    pub to: PageId,
    pub anchor: Option<String>,
}

/// Why a link did not resolve cleanly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Witness {
    BrokenLink {
        from: PageId,
        target: String,
        line: usize,
    },
    AmbiguousLink {
        from: PageId,
        target: String,
        candidates: Vec<PageId>,
        line: usize,
    },
    MissingAnchor {
        from: PageId,
        target: String,
        heading: String,
        line: usize,
    },
}

/// Every resolved link, the backlinks, and every witness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SiteGraph {
    pub resolved: Vec<Resolved>,
    /// For each page, the pages that link to it (sorted, unique).
    pub backlinks: BTreeMap<PageId, BTreeSet<PageId>>,
    pub witnesses: Vec<Witness>,
}

impl SiteGraph {
    /// The resolution of link `index` of page `from`.
    #[must_use]
    pub fn resolution(&self, from: &PageId, index: usize) -> Option<&Resolved> {
        self.resolved
            .iter()
            .find(|r| &r.from == from && r.index == index)
    }
}

fn fold(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// Resolves the links of every document against all of them. Independent of
/// input order (plan §9.4 permutation invariance).
#[must_use]
pub fn resolve(documents: &[Document]) -> SiteGraph {
    let by_id: BTreeMap<&PageId, &Document> = documents.iter().map(|d| (&d.id, d)).collect();
    let mut by_basename: BTreeMap<String, Vec<&PageId>> = BTreeMap::new();
    for id in by_id.keys() {
        by_basename.entry(fold(id.basename())).or_default().push(id);
    }
    let mut graph = SiteGraph::default();
    for doc in by_id.values() {
        for (index, link) in doc.links.iter().enumerate() {
            let target: Option<&Document> = if link.target.is_empty() {
                Some(doc)
            } else if let Some(d) = by_id.get(&PageId::new(&link.target)) {
                Some(d)
            } else {
                match by_basename.get(&fold(&link.target)).map(Vec::as_slice) {
                    Some([only]) => by_id.get(*only).copied(),
                    Some(many) if many.len() > 1 => {
                        graph.witnesses.push(Witness::AmbiguousLink {
                            from: doc.id.clone(),
                            target: link.target.clone(),
                            candidates: many.iter().map(|p| (*p).clone()).collect(),
                            line: link.line,
                        });
                        continue;
                    }
                    _ => None,
                }
            };
            let Some(target) = target else {
                graph.witnesses.push(Witness::BrokenLink {
                    from: doc.id.clone(),
                    target: link.target.clone(),
                    line: link.line,
                });
                continue;
            };
            if let Some(anchor) = &link.anchor
                && !target.headings.iter().any(|h| &h.slug == anchor)
            {
                graph.witnesses.push(Witness::MissingAnchor {
                    from: doc.id.clone(),
                    target: link.target.clone(),
                    heading: anchor.clone(),
                    line: link.line,
                });
            }
            graph
                .backlinks
                .entry(target.id.clone())
                .or_default()
                .insert(doc.id.clone());
            graph.resolved.push(Resolved {
                from: doc.id.clone(),
                index,
                to: target.id.clone(),
                anchor: link.anchor.clone(),
            });
        }
    }
    graph.witnesses.sort();
    graph
}
