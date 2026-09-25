//! Contract suites for the site ports (§9.8). Each returns violations; an
//! empty list means the implementation met the contract on these inputs.

use std::panic::{AssertUnwindSafe, catch_unwind};

use library::RelPath;

use crate::assembly::{PageModel, TocEntry};
use crate::{OutputSink, PageRenderer};

/// A renderer violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererViolation {
    Panicked { page: String },
    NonDeterministic { page: String },
    TocNotObserved { page: String },
    AssetsPanicked,
    AssetsNonDeterministic,
    DuplicatePath { path: String },
}

/// `PageRenderer`: total (no panic), deterministic, unique page/asset paths,
/// and observable TOC sensitivity. The TOC-only counterfactual is a sampled
/// discrimination check; adapter-specific tests own semantic completeness.
#[must_use]
pub fn page_renderer(renderer: &impl PageRenderer, pages: &[PageModel]) -> Vec<RendererViolation> {
    let mut out = Vec::new();
    let mut paths = std::collections::BTreeSet::new();
    for page in pages {
        let once = catch_unwind(AssertUnwindSafe(|| renderer.render(page)));
        let twice = catch_unwind(AssertUnwindSafe(|| renderer.render(page)));
        match (once, twice) {
            (Ok(a), Ok(b)) => {
                if a != b {
                    out.push(RendererViolation::NonDeterministic {
                        page: page.id.to_string(),
                    });
                }
                if !paths.insert(a.path.clone()) {
                    out.push(RendererViolation::DuplicatePath {
                        path: a.path.to_string(),
                    });
                }
                if a == b {
                    let mut toc_probe = page.clone();
                    if toc_probe.toc.is_empty() {
                        toc_probe.toc.push(TocEntry {
                            level: 1,
                            text: "Contract TOC probe".to_owned(),
                            anchor: "contract-toc-probe".to_owned(),
                        });
                    } else {
                        toc_probe.toc.clear();
                    }
                    match catch_unwind(AssertUnwindSafe(|| renderer.render(&toc_probe))) {
                        Ok(probe) if probe.bytes.as_slice() == a.bytes.as_slice() => {
                            out.push(RendererViolation::TocNotObserved {
                                page: page.id.to_string(),
                            });
                        }
                        Ok(_) => {}
                        Err(_) => out.push(RendererViolation::Panicked {
                            page: page.id.to_string(),
                        }),
                    }
                }
            }
            _ => out.push(RendererViolation::Panicked {
                page: page.id.to_string(),
            }),
        }
    }

    let assets_once = catch_unwind(AssertUnwindSafe(|| renderer.assets()));
    let assets_twice = catch_unwind(AssertUnwindSafe(|| renderer.assets()));
    if assets_once.is_err() || assets_twice.is_err() {
        out.push(RendererViolation::AssetsPanicked);
    } else if assets_once.as_ref().ok() != assets_twice.as_ref().ok() {
        out.push(RendererViolation::AssetsNonDeterministic);
    }
    if let Some(assets) = assets_once
        .as_ref()
        .ok()
        .or_else(|| assets_twice.as_ref().ok())
    {
        for asset in assets {
            if !paths.insert(asset.path.clone()) {
                out.push(RendererViolation::DuplicatePath {
                    path: asset.path.to_string(),
                });
            }
        }
    }
    out
}

/// A sink violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkViolation {
    Failed(String),
    WriteNotListed(String),
    DigestWrong(String),
    DeleteStillListed(String),
}

/// `OutputSink`: a completed write is listed with its digest; a delete
/// removes it. Uses a probe path under `__contract__/`.
pub fn output_sink(sink: &mut impl OutputSink) -> Vec<SinkViolation> {
    let mut out = Vec::new();
    let Ok(path) = RelPath::new("__contract__/probe.bin") else {
        return out;
    };
    let bytes = b"contract probe \x00\xff";
    if let Err(e) = sink.write(&path, bytes) {
        return vec![SinkViolation::Failed(e.message)];
    }
    match sink.list() {
        Ok(list) => match list.iter().find(|(p, _)| p == &path) {
            Some((_, d)) if *d == library::Digest::of(bytes) => {}
            Some(_) => out.push(SinkViolation::DigestWrong(path.to_string())),
            None => out.push(SinkViolation::WriteNotListed(path.to_string())),
        },
        Err(e) => out.push(SinkViolation::Failed(e.message)),
    }
    if let Err(e) = sink.delete(&path) {
        out.push(SinkViolation::Failed(e.message));
    } else {
        match sink.list() {
            Ok(list) if list.iter().any(|(p, _)| p == &path) => {
                out.push(SinkViolation::DeleteStillListed(path.to_string()));
            }
            Ok(_) => {}
            Err(e) => out.push(SinkViolation::Failed(e.message)),
        }
    }
    out
}
