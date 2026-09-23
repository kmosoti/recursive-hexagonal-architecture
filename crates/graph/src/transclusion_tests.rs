#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};

use library::PageId;
use serde_json::{Value, json};

use super::transclusion::{
    TransclusionCycle, TransclusionOccurrence, TransclusionRegion, analyze_regions,
};

#[path = "../../../xtask/tests/support/registered_package.rs"]
mod registered_package;

fn region(value: &Value) -> TransclusionRegion {
    TransclusionRegion {
        page: PageId::new(value["page"].as_str().expect("region page")),
        anchor: value["anchor"].as_str().map(str::to_owned),
    }
}

fn graph(
    case: &Value,
) -> (
    BTreeSet<TransclusionRegion>,
    BTreeMap<TransclusionOccurrence, TransclusionRegion>,
) {
    let vertices = case["vertices"]
        .as_array()
        .expect("vertices")
        .iter()
        .map(region)
        .collect();
    let edges = case["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| {
            let occurrence = TransclusionOccurrence {
                source: region(&edge["source"]),
                transclusion_id: edge["transclusion_id"]
                    .as_u64()
                    .expect("transclusion id")
                    .try_into()
                    .expect("transclusion id conversion"),
            };
            (occurrence, region(&edge["target"]))
        })
        .collect();
    (vertices, edges)
}

fn region_json(region: &TransclusionRegion) -> Value {
    json!({
        "page": region.page.as_str(),
        "anchor": region.anchor,
    })
}

fn occurrence_json(occurrence: &TransclusionOccurrence) -> Value {
    json!({
        "source": region_json(&occurrence.source),
        "transclusion_id": occurrence.transclusion_id,
    })
}

fn cycle_json(cycle: &TransclusionCycle) -> Value {
    Value::Array(cycle.path.iter().map(region_json).collect())
}

fn result_json(
    blocked: &BTreeMap<TransclusionOccurrence, TransclusionCycle>,
    cycles: &[TransclusionCycle],
) -> Value {
    json!({
        "blocked": blocked
            .iter()
            .map(|(occurrence, cycle)| {
                json!({
                    "source": occurrence_json(occurrence)["source"],
                    "transclusion_id": occurrence.transclusion_id,
                    "path": cycle_json(cycle),
                })
            })
            .collect::<Vec<_>>(),
        "cycles": cycles.iter().map(cycle_json).collect::<Vec<_>>(),
    })
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LocalRegion {
    page: String,
    anchor: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LocalOccurrence {
    source: LocalRegion,
    transclusion_id: usize,
}

fn local_region(value: &Value) -> LocalRegion {
    LocalRegion {
        page: value["page"].as_str().expect("local page").to_owned(),
        anchor: value["anchor"].as_str().map(str::to_owned),
    }
}

fn dag_after_blocking(case: &Value) -> bool {
    let vertices = case["vertices"]
        .as_array()
        .expect("vertices")
        .iter()
        .map(local_region)
        .collect::<BTreeSet<_>>();
    let blocked = case["expected"]["blocked"]
        .as_array()
        .expect("blocked")
        .iter()
        .map(|entry| LocalOccurrence {
            source: local_region(&entry["source"]),
            transclusion_id: entry["transclusion_id"]
                .as_u64()
                .expect("blocked id")
                .try_into()
                .expect("blocked id conversion"),
        })
        .collect::<BTreeSet<_>>();

    let mut indegree = vertices
        .iter()
        .cloned()
        .map(|vertex| (vertex, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<LocalRegion, Vec<(LocalOccurrence, LocalRegion)>>::new();

    for edge in case["edges"].as_array().expect("edges") {
        let occurrence = LocalOccurrence {
            source: local_region(&edge["source"]),
            transclusion_id: edge["transclusion_id"]
                .as_u64()
                .expect("edge id")
                .try_into()
                .expect("edge id conversion"),
        };
        if blocked.contains(&occurrence) {
            continue;
        }
        let target = local_region(&edge["target"]);
        assert!(vertices.contains(&occurrence.source));
        assert!(vertices.contains(&target));
        *indegree.get_mut(&target).expect("target vertex") += 1;
        outgoing
            .entry(occurrence.source.clone())
            .or_default()
            .push((occurrence, target));
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(vertex, degree)| (*degree == 0).then_some(vertex.clone()))
        .collect::<BTreeSet<_>>();
    let mut visited = 0usize;
    while let Some(vertex) = ready.pop_first() {
        visited += 1;
        if let Some(edges) = outgoing.get(&vertex) {
            for (_, target) in edges {
                let degree = indegree.get_mut(target).expect("target degree");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(target.clone());
                }
            }
        }
    }
    visited == vertices.len()
}

fn reversed(value: &Value, key: &str) -> Value {
    let mut values = value[key].as_array().expect(key).clone();
    values.reverse();
    Value::Array(values)
}

#[test]
fn all_registered_region_graphs_match_exact_blocking_and_cycle_witnesses() {
    let cases = registered_package::load("regions");
    assert_eq!(cases.len(), 262);

    for case in &cases {
        let (vertices, edges) = graph(case);
        let (blocked, cycles) = analyze_regions(&vertices, &edges);
        assert_eq!(
            result_json(&blocked, &cycles),
            json!({
                "blocked": case["expected"]["blocked"],
                "cycles": case["expected"]["cycles"],
            }),
            "{}",
            case["id"]
        );
        assert_eq!(
            dag_after_blocking(case),
            case["expected"]["remaining_is_dag"]
                .as_bool()
                .expect("remaining_is_dag"),
            "{}: independent DAG check",
            case["id"]
        );

        let permuted = json!({
            "vertices": reversed(case, "vertices"),
            "edges": reversed(case, "edges"),
        });
        let (permuted_vertices, permuted_edges) = graph(&permuted);
        let (permuted_blocked, permuted_cycles) =
            analyze_regions(&permuted_vertices, &permuted_edges);
        assert_eq!(
            result_json(&permuted_blocked, &permuted_cycles),
            result_json(&blocked, &cycles),
            "{}: permutation differential",
            case["id"]
        );
    }
}
