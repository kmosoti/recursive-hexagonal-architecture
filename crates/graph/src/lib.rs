//! `graph`: resolve every wikilink or witness why not (plan §3.2).
//!
//! Resolution (the registered contract): an exact page id wins; otherwise the
//! unique page whose basename equals the target case-insensitively;
//! otherwise the link is broken, or ambiguous when several match. An anchor
//! must equal one of the target page's final slugs.
#![forbid(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::disallowed_macros
)]

use std::collections::{BTreeMap, BTreeSet};

use document::{Document, section_nodes, transclusions};
use library::PageId;

mod transclusion;
pub use transclusion::{TransclusionCycle, TransclusionOccurrence, TransclusionRegion};

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
    BrokenTransclusion {
        from: PageId,
        target: String,
        line: usize,
    },
    AmbiguousTransclusion {
        from: PageId,
        target: String,
        candidates: Vec<PageId>,
        line: usize,
    },
    MissingTransclusionAnchor {
        from: PageId,
        target: String,
        heading: String,
        line: usize,
    },
    TransclusionCycle {
        path: Vec<TransclusionRegion>,
    },
}

/// Every resolved link, the backlinks, and every witness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SiteGraph {
    pub resolved: Vec<Resolved>,
    /// For each page, the pages that link to it (sorted, unique).
    pub backlinks: BTreeMap<PageId, BTreeSet<PageId>>,
    pub witnesses: Vec<Witness>,
    transclusion_targets: BTreeMap<TransclusionOccurrence, TransclusionRegion>,
    blocked_transclusions: BTreeMap<TransclusionOccurrence, TransclusionCycle>,
}

impl SiteGraph {
    /// The resolution of link `index` of page `from`.
    #[must_use]
    pub fn resolution(&self, from: &PageId, index: usize) -> Option<&Resolved> {
        self.resolved
            .iter()
            .find(|r| &r.from == from && r.index == index)
    }

    /// The resolved target of a transclusion occurrence.
    #[must_use]
    pub fn transclusion_target(
        &self,
        occurrence: &TransclusionOccurrence,
    ) -> Option<&TransclusionRegion> {
        self.transclusion_targets.get(occurrence)
    }

    /// The cycle blocking a transclusion occurrence, if any.
    #[must_use]
    pub fn blocked_transclusion(
        &self,
        occurrence: &TransclusionOccurrence,
    ) -> Option<&TransclusionCycle> {
        self.blocked_transclusions.get(occurrence)
    }
}

fn fold(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

enum TargetResolution<'a> {
    Resolved(&'a Document),
    Broken,
    Ambiguous(Vec<PageId>),
}

fn resolve_target<'a>(
    source: &'a Document,
    target: &str,
    by_id: &BTreeMap<&'a PageId, &'a Document>,
    by_basename: &BTreeMap<String, Vec<&'a PageId>>,
) -> TargetResolution<'a> {
    if target.is_empty() {
        return TargetResolution::Resolved(source);
    }
    let exact = PageId::new(target);
    if let Some(document) = by_id.get(&exact) {
        return TargetResolution::Resolved(document);
    }
    match by_basename.get(&fold(target)).map(Vec::as_slice) {
        Some([only]) => TargetResolution::Resolved(by_id[only]),
        Some(many) if many.len() > 1 => {
            TargetResolution::Ambiguous(many.iter().map(|page| (*page).clone()).collect())
        }
        _ => TargetResolution::Broken,
    }
}

/// Resolves the links of every document against all of them. Documents must
/// have unique page ids, as enforced by `library::Corpus`. Within that input
/// domain, resolution is independent of order (plan §9.4 permutation invariance).
#[must_use]
pub fn resolve(documents: &[Document]) -> SiteGraph {
    let by_id: BTreeMap<&PageId, &Document> = documents.iter().map(|d| (&d.id, d)).collect();
    let mut by_basename: BTreeMap<String, Vec<&PageId>> = BTreeMap::new();
    for id in by_id.keys() {
        by_basename.entry(fold(id.basename())).or_default().push(id);
    }
    let mut graph = SiteGraph::default();
    for &doc in by_id.values() {
        for (index, link) in doc.links.iter().enumerate() {
            match resolve_target(doc, &link.target, &by_id, &by_basename) {
                TargetResolution::Resolved(target) => {
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
                TargetResolution::Broken => {
                    graph.witnesses.push(Witness::BrokenLink {
                        from: doc.id.clone(),
                        target: link.target.clone(),
                        line: link.line,
                    });
                }
                TargetResolution::Ambiguous(candidates) => {
                    graph.witnesses.push(Witness::AmbiguousLink {
                        from: doc.id.clone(),
                        target: link.target.clone(),
                        candidates,
                        line: link.line,
                    });
                }
            }
        }
    }

    let mut transclusion_targets = BTreeMap::new();
    let mut regions = BTreeSet::new();
    let mut pending = Vec::new();

    for &doc in by_id.values() {
        let full = TransclusionRegion {
            page: doc.id.clone(),
            anchor: None,
        };
        regions.insert(full.clone());
        pending.push(full);

        for transclusion in transclusions(&doc.body) {
            match resolve_target(doc, &transclusion.target, &by_id, &by_basename) {
                TargetResolution::Resolved(target) => {
                    transclusion_targets
                        .insert((doc.id.clone(), transclusion.id), target.id.clone());
                    graph
                        .backlinks
                        .entry(target.id.clone())
                        .or_default()
                        .insert(doc.id.clone());

                    if let Some(anchor) = &transclusion.anchor {
                        if section_nodes(target, Some(anchor.as_str())).is_some() {
                            let region = TransclusionRegion {
                                page: target.id.clone(),
                                anchor: Some(anchor.clone()),
                            };
                            if regions.insert(region.clone()) {
                                pending.push(region);
                            }
                        } else {
                            graph.witnesses.push(Witness::MissingTransclusionAnchor {
                                from: doc.id.clone(),
                                target: transclusion.target.clone(),
                                heading: anchor.clone(),
                                line: transclusion.line,
                            });
                        }
                    }
                }
                TargetResolution::Broken => {
                    graph.witnesses.push(Witness::BrokenTransclusion {
                        from: doc.id.clone(),
                        target: transclusion.target.clone(),
                        line: transclusion.line,
                    });
                }
                TargetResolution::Ambiguous(candidates) => {
                    graph.witnesses.push(Witness::AmbiguousTransclusion {
                        from: doc.id.clone(),
                        target: transclusion.target.clone(),
                        candidates,
                        line: transclusion.line,
                    });
                }
            }
        }
    }

    let mut cursor = 0;
    while cursor < pending.len() {
        let source = pending[cursor].clone();
        cursor += 1;
        let Some(doc) = by_id.get(&source.page).copied() else {
            continue;
        };
        let Some(nodes) = section_nodes(doc, source.anchor.as_deref()) else {
            continue;
        };
        for transclusion in transclusions(&nodes) {
            let Some(target_page) = transclusion_targets.get(&(doc.id.clone(), transclusion.id))
            else {
                continue;
            };
            let Some(anchor) = &transclusion.anchor else {
                continue;
            };
            let Some(target) = by_id.get(target_page).copied() else {
                continue;
            };
            if section_nodes(target, Some(anchor.as_str())).is_none() {
                continue;
            }
            let region = TransclusionRegion {
                page: target_page.clone(),
                anchor: Some(anchor.clone()),
            };
            if regions.insert(region.clone()) {
                pending.push(region);
            }
        }
    }

    let mut edges = BTreeMap::new();
    for source in &regions {
        let Some(doc) = by_id.get(&source.page).copied() else {
            continue;
        };
        let Some(nodes) = section_nodes(doc, source.anchor.as_deref()) else {
            continue;
        };
        for transclusion in transclusions(&nodes) {
            let Some(target_page) = transclusion_targets.get(&(doc.id.clone(), transclusion.id))
            else {
                continue;
            };
            let target = TransclusionRegion {
                page: target_page.clone(),
                anchor: transclusion.anchor.clone(),
            };
            if !regions.contains(&target) {
                continue;
            }
            let occurrence = TransclusionOccurrence {
                source: source.clone(),
                transclusion_id: transclusion.id,
            };
            graph
                .transclusion_targets
                .insert(occurrence.clone(), target.clone());
            edges.insert(occurrence, target);
        }
    }

    let (blocked, cycles) = transclusion::analyze_regions(&regions, &edges);
    graph.blocked_transclusions = blocked;
    graph.witnesses.extend(
        cycles
            .into_iter()
            .map(|cycle| Witness::TransclusionCycle { path: cycle.path }),
    );
    graph.witnesses.sort();
    graph
}

#[cfg(test)]
mod transclusion_tests;
