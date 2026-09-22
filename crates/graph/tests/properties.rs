//! Plan §3.2: every link resolves or is witnessed; backlinks are symmetric;
//! resolution is independent of input order (§9.4). The witness check is
//! independent of `graph`'s code (plan §3.2, last column).

use document::parse;
use graph::{Witness, resolve};
use library::{RelPath, Source};
use proptest::prelude::*;

fn doc(path: &str, text: &str) -> document::Document {
    parse(&Source::new(
        RelPath::new(path).expect("valid"),
        text.to_owned(),
    ))
}

fn site() -> impl Strategy<Value = Vec<(String, String)>> {
    let names = prop::sample::select(vec!["a", "b", "x/a", "x/C", "c"]);
    let targets = prop::sample::select(vec![
        "a", "A", "b", "c", "x/a", "zz", "C#one", "b#two", "#one",
    ]);
    prop::collection::vec((names, prop::collection::vec(targets, 0..4)), 1..5).prop_map(|pages| {
        let mut seen = std::collections::BTreeSet::new();
        pages
            .into_iter()
            .filter(|(n, _)| seen.insert(*n))
            .map(|(n, ts)| {
                let body: String = ts.iter().map(|t| format!("[[{t}]] ")).collect();
                (format!("{n}.md"), format!("# One\n\n{body}\n"))
            })
            .collect()
    })
}

proptest! {
    #[test]
    fn every_link_resolves_or_is_witnessed_and_backlinks_are_symmetric(pages in site()) {
        let docs: Vec<_> = pages.iter().map(|(p, t)| doc(p, t)).collect();
        let g = resolve(&docs);
        let links: usize = docs.iter().map(|d| d.links.len()).sum();
        let unresolved = g.witnesses.iter().filter(|w| !matches!(w, Witness::MissingAnchor { .. })).count();
        prop_assert_eq!(g.resolved.len() + unresolved, links);
        for r in &g.resolved {
            prop_assert!(g.backlinks.get(&r.to).is_some_and(|s| s.contains(&r.from)));
        }
        for (to, sources) in &g.backlinks {
            for from in sources {
                prop_assert!(g.resolved.iter().any(|r| &r.to == to && &r.from == from));
            }
        }
    }

    #[test]
    fn resolution_is_independent_of_order(pages in site()) {
        let mut docs: Vec<_> = pages.iter().map(|(p, t)| doc(p, t)).collect();
        let first = resolve(&docs);
        docs.reverse();
        let second = resolve(&docs);
        prop_assert_eq!(first.witnesses, second.witnesses);
        prop_assert_eq!(first.backlinks, second.backlinks);
    }
}

#[test]
fn exact_then_unique_basename_then_broken_or_ambiguous() {
    let docs = vec![
        doc("a.md", "[[x/b]] [[B]] [[q]] [[c]]"),
        doc("x/b.md", "# T"),
        doc("x/c.md", ""),
        doc("y/c.md", ""),
    ];
    let g = resolve(&docs);
    assert_eq!(
        g.resolved.iter().filter(|r| r.from.as_str() == "a").count(),
        2
    );
    assert!(
        g.witnesses
            .iter()
            .any(|w| matches!(w, Witness::BrokenLink { target, .. } if target == "q"))
    );
    assert!(
        g.witnesses
            .iter()
            .any(|w| matches!(w, Witness::AmbiguousLink { target, .. } if target == "c"))
    );
}
