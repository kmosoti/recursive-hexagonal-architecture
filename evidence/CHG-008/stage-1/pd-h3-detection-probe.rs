use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use library::{Corpus, Digest, PageId, RelPath, Source};
use site::assembly::{Assemble, AssembleContext, DefaultAssembler, PageModel};
use site::contract::{RendererViolation, SinkViolation, output_sink, page_renderer};
use site::testing::{FixedClock, RecordingSink, StubRenderer};
use site::{OutputSink, PageRenderer, Rendered, SinkError, analyse, build_all};

fn corpus() -> Corpus {
    Corpus::new(vec![
        Source::new(
            RelPath::new("a.md").expect("valid"),
            "# A\n[[b]]".to_owned(),
        ),
        Source::new(
            RelPath::new("b.md").expect("valid"),
            "## B\n[[a]]".to_owned(),
        ),
    ])
    .expect("unique page ids")
}

fn pages(corpus: &Corpus) -> Vec<PageModel> {
    let (documents, graph) = analyse(corpus);
    let no_titles = |_: &PageId| None;
    documents
        .iter()
        .map(|document| {
            DefaultAssembler.assemble(
                document,
                &graph,
                &no_titles,
                &AssembleContext {
                    built_at: Some("t0".to_owned()),
                },
            )
        })
        .collect()
}

fn page_path(id: &PageId) -> RelPath {
    RelPath::new(&format!("{id}.html")).expect("source-derived page id is a valid path")
}

fn debug_page(page: &PageModel) -> Vec<u8> {
    format!("{page:?}").into_bytes()
}

#[derive(Debug, Clone, Copy, Default)]
struct GoodEchoRenderer;

impl PageRenderer for GoodEchoRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        Rendered {
            path: page_path(&page.id),
            bytes: debug_page(page),
        }
    }

    fn assets(&self) -> Vec<Rendered> {
        vec![
            Rendered {
                path: RelPath::new("_assets/echo.css").expect("valid"),
                bytes: b"echo-css".to_vec(),
            },
            Rendered {
                path: RelPath::new("_assets/echo.js").expect("valid"),
                bytes: b"echo-js".to_vec(),
            },
        ]
    }
}

struct CellRenderer {
    counter: Cell<u8>,
}

impl PageRenderer for CellRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        self.counter
            .set(self.counter.get().wrapping_add(1));
        Rendered {
            path: page_path(&page.id),
            bytes: vec![self.counter.get()],
        }
    }

    fn assets(&self) -> Vec<Rendered> {
        Vec::new()
    }
}

struct PanickingRenderer;

impl PageRenderer for PanickingRenderer {
    fn render(&self, _: &PageModel) -> Rendered {
        panic!("render panic")
    }

    fn assets(&self) -> Vec<Rendered> {
        Vec::new()
    }
}

struct TocDroppingRenderer;

impl PageRenderer for TocDroppingRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        let mut dropped = page.clone();
        dropped.toc.clear();
        GoodEchoRenderer.render(&dropped)
    }

    fn assets(&self) -> Vec<Rendered> {
        GoodEchoRenderer.assets()
    }
}

struct PanickingAssetsRenderer;

impl PageRenderer for PanickingAssetsRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        GoodEchoRenderer.render(page)
    }

    fn assets(&self) -> Vec<Rendered> {
        panic!("assets panic")
    }
}

struct AssetCollidesWithPage;

impl PageRenderer for AssetCollidesWithPage {
    fn render(&self, page: &PageModel) -> Rendered {
        GoodEchoRenderer.render(page)
    }

    fn assets(&self) -> Vec<Rendered> {
        vec![Rendered {
            path: RelPath::new("a.html").expect("valid"),
            bytes: b"collision".to_vec(),
        }]
    }
}

struct DuplicateAssets;

impl PageRenderer for DuplicateAssets {
    fn render(&self, page: &PageModel) -> Rendered {
        GoodEchoRenderer.render(page)
    }

    fn assets(&self) -> Vec<Rendered> {
        vec![
            Rendered {
                path: RelPath::new("_assets/shared.css").expect("valid"),
                bytes: b"first".to_vec(),
            },
            Rendered {
                path: RelPath::new("_assets/shared.css").expect("valid"),
                bytes: b"second".to_vec(),
            },
        ]
    }
}

struct ParametricRenderer {
    asset_byte: u8,
}

impl PageRenderer for ParametricRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        Rendered {
            path: page_path(&page.id),
            bytes: debug_page(page),
        }
    }

    fn assets(&self) -> Vec<Rendered> {
        vec![Rendered {
            path: RelPath::new("_assets/site.css").expect("valid"),
            bytes: vec![self.asset_byte],
        }]
    }
}

#[derive(Default)]
struct FailAfterDelete {
    inner: RecordingSink,
    deleted: bool,
}

impl OutputSink for FailAfterDelete {
    fn list(&self) -> Result<Vec<(RelPath, Digest)>, SinkError> {
        if self.deleted {
            Err(SinkError {
                path: None,
                message: "listing failed after delete".to_owned(),
            })
        } else {
            self.inner.list()
        }
    }

    fn write(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), SinkError> {
        self.inner.write(path, bytes)
    }

    fn delete(&mut self, path: &RelPath) -> Result<(), SinkError> {
        let result = self.inner.delete(path);
        if result.is_ok() {
            self.deleted = true;
        }
        result
    }
}

#[test]
fn stable_debug_echo_renderer_has_no_contract_violations() {
    let models = pages(&corpus());

    assert!(page_renderer(&GoodEchoRenderer, &models).is_empty());
}

#[test]
fn renderer_contract_reports_nondeterminism_and_panics() {
    let models = pages(&corpus());

    let drifting = page_renderer(
        &CellRenderer {
            counter: Cell::new(0),
        },
        &models,
    );
    assert!(
        drifting
            .iter()
            .any(|v| matches!(v, RendererViolation::NonDeterministic { .. })),
        "{drifting:?}"
    );

    let panicking = page_renderer(&PanickingRenderer, &models);
    assert!(
        panicking
            .iter()
            .all(|v| matches!(v, RendererViolation::Panicked { .. })),
        "{panicking:?}"
    );
}

#[test]
fn existing_positive_contract_controls_remain_empty() {
    let models = pages(&corpus());

    assert!(page_renderer(&StubRenderer, &models).is_empty());
    assert!(output_sink(&mut RecordingSink::default()).is_empty());
}

#[test]
fn renderer_contract_detects_a_renderer_that_drops_toc() {
    let models = pages(&corpus());
    assert!(models.iter().any(|page| !page.toc.is_empty()));

    let violations = page_renderer(&TocDroppingRenderer, &models);
    assert!(!violations.is_empty(), "{violations:?}");
}

#[test]
fn renderer_contract_contains_asset_panics() {
    let models = pages(&corpus());
    let result = catch_unwind(AssertUnwindSafe(|| {
        page_renderer(&PanickingAssetsRenderer, &models)
    }));

    let violations = result.expect("asset panic must not escape page_renderer");
    assert!(!violations.is_empty(), "{violations:?}");
}

#[test]
fn renderer_contract_reports_page_asset_collisions_and_duplicate_assets() {
    let models = pages(&corpus());

    let collision = page_renderer(&AssetCollidesWithPage, &models);
    assert!(
        collision.iter().any(
            |v| matches!(v, RendererViolation::DuplicatePath { path } if path == "a.html")
        ),
        "{collision:?}"
    );

    let duplicate_assets = page_renderer(&DuplicateAssets, &models);
    assert!(
        duplicate_assets.iter().any(
            |v| matches!(v, RendererViolation::DuplicatePath { path } if path == "_assets/shared.css")
        ),
        "{duplicate_assets:?}"
    );
}

#[test]
fn build_counts_unchanged_pages_when_only_an_asset_changes() {
    let corpus = corpus();
    let mut sink = RecordingSink::default();

    let first = build_all(
        &corpus,
        &FixedClock("t0".to_owned()),
        &mut sink,
        &ParametricRenderer { asset_byte: 1 },
    )
    .expect("first build");
    assert_eq!(first.written.len(), 3, "{first:?}");
    assert_eq!(first.unchanged, 0, "{first:?}");

    let asset_path = RelPath::new("_assets/site.css").expect("valid");
    let second = build_all(
        &corpus,
        &FixedClock("t0".to_owned()),
        &mut sink,
        &ParametricRenderer { asset_byte: 2 },
    )
    .expect("second build");
    assert_eq!(second.written, vec![asset_path], "{second:?}");
    assert!(second.deleted.is_empty(), "{second:?}");
    assert_eq!(second.unchanged, 2, "{second:?}");
}

#[test]
fn output_sink_contract_reports_postdelete_listing_failure() {
    let violations = output_sink(&mut FailAfterDelete::default());

    assert!(
        violations.iter().any(|violation| matches!(
            violation,
            SinkViolation::Failed(message) if message == "listing failed after delete"
        )),
        "{violations:?}"
    );
}

fn main() {
 let models=pages(&corpus());
 let unstable=page_renderer(&CellRenderer{counter:Cell::new(0)},&models);
 println!("nondeterministic renderer: {unstable:?}");
 assert!(unstable.iter().any(|v|matches!(v,RendererViolation::NonDeterministic{..})));
 let panicking=page_renderer(&PanickingRenderer,&models);
 println!("panicking renderer: {panicking:?}");
 assert!(!panicking.is_empty() && panicking.iter().all(|v|matches!(v,RendererViolation::Panicked{..})));
 let dropping=page_renderer(&TocDroppingRenderer,&models);
 println!("TOC-dropping renderer: {dropping:?}");
 assert!(dropping.iter().any(|v|matches!(v,RendererViolation::TocNotObserved{..})));
 let json=adapter_json::JsonRenderer::new(&models).unwrap();
 for(name,violations) in [("StubRenderer",page_renderer(&StubRenderer,&models)),("HtmlRenderer",page_renderer(&adapter_html::HtmlRenderer,&models)),("JsonRenderer",page_renderer(&json,&models))] {
  println!("{name}: {violations:?}"); assert!(violations.is_empty());
 }
}
