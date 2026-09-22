//! Plan §3.2: `Inv_K`; assemble deterministic; the build converges.

use library::testing::MemorySources;
use library::{RelPath, load};
use site::assembly::{Assemble, AssembleContext, DefaultAssembler};
use site::build::{BuildInputs, Command, Observation, SiteState, check_inv_k, step};
use site::contract::{RendererViolation, output_sink, page_renderer};
use site::testing::{FixedClock, RecordingSink, StubRenderer};
use site::{PageRenderer, Rendered, analyse, build_all};

fn corpus() -> library::Corpus {
    load(&MemorySources::new(&[
        ("a.md", "# A\n[[b]] [[zz]]"),
        ("x/b.md", "# B\n## One\n[[a#a]]"),
    ]))
    .expect("loads")
    .0
}

#[test]
fn build_writes_then_converges_and_deletes_stale_outputs() {
    let mut sink = RecordingSink::default();
    sink.files
        .insert(RelPath::new("stale.txt").expect("valid"), b"old".to_vec());
    let first = build_all(
        &corpus(),
        &FixedClock("t0".into()),
        &mut sink,
        &StubRenderer,
    )
    .expect("builds");
    assert_eq!(first.written.len(), 2);
    assert_eq!(
        first.deleted,
        vec![RelPath::new("stale.txt").expect("valid")]
    );
    assert_eq!(first.link_witnesses.len(), 1, "zz is broken");
    let second = build_all(
        &corpus(),
        &FixedClock("t0".into()),
        &mut sink,
        &StubRenderer,
    )
    .expect("builds");
    assert!(
        second.written.is_empty() && second.deleted.is_empty(),
        "{second:?}"
    );
}

#[test]
fn inv_k_rejects_a_write_for_no_page_and_a_delete_of_nothing() {
    let (docs, graph) = analyse(&corpus());
    let inputs = BuildInputs {
        documents: &docs,
        graph: &graph,
        context: AssembleContext::default(),
    };
    let bad_write = vec![Command::Write {
        path: RelPath::new("q.txt").expect("valid"),
        bytes: vec![],
        page: Some(library::PageId::new("ghost")),
    }];
    assert!(check_inv_k(&bad_write, &Observation::default(), &inputs).is_err());
    let bad_delete = vec![Command::Delete {
        path: RelPath::new("never.txt").expect("valid"),
    }];
    assert!(check_inv_k(&bad_delete, &Observation::default(), &inputs).is_err());
    let (_, ok) = step(
        &SiteState::default(),
        &Observation::default(),
        &inputs,
        &DefaultAssembler,
        &StubRenderer,
    )
    .expect("steps");
    assert!(check_inv_k(&ok, &Observation::default(), &inputs).is_ok());
}

#[test]
fn assembly_is_deterministic() {
    let (docs, graph) = analyse(&corpus());
    let ctx = AssembleContext {
        built_at: Some("t".into()),
    };
    let none = |_: &library::PageId| None;
    for doc in &docs {
        assert_eq!(
            DefaultAssembler.assemble(doc, &graph, &none, &ctx),
            DefaultAssembler.assemble(doc, &graph, &none, &ctx)
        );
    }
}

struct Panicky;
impl PageRenderer for Panicky {
    fn render(&self, _: &site::assembly::PageModel) -> Rendered {
        panic!("seeded panicking renderer")
    }
    fn assets(&self) -> Vec<Rendered> {
        Vec::new()
    }
}

struct Drifting(std::cell::Cell<u8>);
impl PageRenderer for Drifting {
    fn render(&self, p: &site::assembly::PageModel) -> Rendered {
        self.0.set(self.0.get().wrapping_add(1));
        Rendered {
            path: RelPath::new(&format!("{}.txt", p.id)).expect("valid"),
            bytes: vec![self.0.get()],
        }
    }
    fn assets(&self) -> Vec<Rendered> {
        Vec::new()
    }
}

#[test]
fn the_renderer_and_sink_contracts_catch_seeded_violators_and_pass_the_fakes() {
    let (docs, graph) = analyse(&corpus());
    let none = |_: &library::PageId| None;
    let pages: Vec<_> = docs
        .iter()
        .map(|d| DefaultAssembler.assemble(d, &graph, &none, &AssembleContext::default()))
        .collect();
    assert!(page_renderer(&StubRenderer, &pages).is_empty());
    assert!(
        page_renderer(&Panicky, &pages)
            .iter()
            .all(|v| matches!(v, RendererViolation::Panicked { .. }))
    );
    assert!(
        page_renderer(&Drifting(std::cell::Cell::new(0)), &pages)
            .iter()
            .any(|v| matches!(v, RendererViolation::NonDeterministic { .. }))
    );
    assert!(output_sink(&mut RecordingSink::default()).is_empty());
}
