//! Contract suites for the site ports (§9.8). Each returns violations; an
//! empty list means the implementation met the contract on these inputs.

use std::panic::{AssertUnwindSafe, catch_unwind};

use library::RelPath;

use crate::assembly::PageModel;
use crate::{OutputSink, PageRenderer};

/// A renderer violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererViolation {
    Panicked { page: String },
    NonDeterministic { page: String },
    AssetsNonDeterministic,
    DuplicatePath { path: String },
}

/// `PageRenderer`: total (no panic), deterministic, one path per page.
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
            }
            _ => out.push(RendererViolation::Panicked {
                page: page.id.to_string(),
            }),
        }
    }
    if renderer.assets() != renderer.assets() {
        out.push(RendererViolation::AssetsNonDeterministic);
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
    } else if sink.list().is_ok_and(|l| l.iter().any(|(p, _)| p == &path)) {
        out.push(SinkViolation::DeleteStillListed(path.to_string()));
    }
    out
}
