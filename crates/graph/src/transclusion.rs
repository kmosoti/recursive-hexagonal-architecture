use std::collections::{BTreeMap, BTreeSet};

use library::PageId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransclusionRegion {
    pub page: PageId,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransclusionOccurrence {
    pub source: TransclusionRegion,
    pub transclusion_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransclusionCycle {
    pub path: Vec<TransclusionRegion>,
}

pub(super) fn analyze_regions(
    vertices: &BTreeSet<TransclusionRegion>,
    edges: &BTreeMap<TransclusionOccurrence, TransclusionRegion>,
) -> (
    BTreeMap<TransclusionOccurrence, TransclusionCycle>,
    Vec<TransclusionCycle>,
) {
    let mut outgoing: BTreeMap<
        TransclusionRegion,
        Vec<(TransclusionOccurrence, TransclusionRegion)>,
    > = BTreeMap::new();
    for (occurrence, target) in edges {
        if vertices.contains(&occurrence.source) && vertices.contains(target) {
            outgoing
                .entry(occurrence.source.clone())
                .or_default()
                .push((occurrence.clone(), target.clone()));
        }
    }
    for edges in outgoing.values_mut() {
        edges.sort_by(|left, right| {
            left.0
                .transclusion_id
                .cmp(&right.0.transclusion_id)
                .then_with(|| left.1.cmp(&right.1))
        });
    }

    let mut visited = BTreeSet::new();
    let mut active = Vec::new();
    let mut blocked = BTreeMap::new();
    let mut cycles = BTreeSet::new();

    for root in vertices {
        if visited.contains(root) {
            continue;
        }
        visit(
            root,
            &outgoing,
            &mut visited,
            &mut active,
            &mut blocked,
            &mut cycles,
        );
    }

    (blocked, cycles.into_iter().collect())
}

fn visit(
    current: &TransclusionRegion,
    outgoing: &BTreeMap<TransclusionRegion, Vec<(TransclusionOccurrence, TransclusionRegion)>>,
    visited: &mut BTreeSet<TransclusionRegion>,
    active: &mut Vec<TransclusionRegion>,
    blocked: &mut BTreeMap<TransclusionOccurrence, TransclusionCycle>,
    cycles: &mut BTreeSet<TransclusionCycle>,
) {
    active.push(current.clone());
    if let Some(edges) = outgoing.get(current) {
        for (occurrence, target) in edges {
            if let Some(start) = active.iter().position(|region| region == target) {
                let mut path = active[start..].to_vec();
                path.push(target.clone());
                let cycle = TransclusionCycle {
                    path: canonical_cycle(&path),
                };
                blocked.insert(occurrence.clone(), cycle.clone());
                cycles.insert(cycle);
            } else if !visited.contains(target) {
                visit(target, outgoing, visited, active, blocked, cycles);
            }
        }
    }
    active.pop();
    visited.insert(current.clone());
}

fn canonical_cycle(path: &[TransclusionRegion]) -> Vec<TransclusionRegion> {
    if path.len() < 2 {
        return path.to_vec();
    }
    let cycle_len = path.len() - 1;
    let mut candidates = Vec::with_capacity(cycle_len);
    for start in 0..cycle_len {
        let mut candidate = Vec::with_capacity(path.len());
        for offset in 0..cycle_len {
            candidate.push(path[(start + offset) % cycle_len].clone());
        }
        let first = candidate[0].clone();
        candidate.push(first);
        candidates.push(candidate);
    }
    candidates.into_iter().min().unwrap_or_default()
}
