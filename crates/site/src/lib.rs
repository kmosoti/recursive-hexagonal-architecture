//! `site`: page assembly and the pure build step (children), glued to the
//! output and clock ports (BDR-0003).
#![forbid(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::disallowed_macros
)]

pub mod assembly;
pub mod build;
pub mod contract;
mod port;
pub mod testing;

pub use build::{PageRenderer, Rendered};
/// What a renderer needs from the owners of the model, re-exported so that a
/// rendering adapter depends on its port's owner only (D1; no
/// `adapter.foreign_core`).
pub use document::{Align, CalloutKind, Node};
pub use library::{PageId, RelPath};
pub use port::{Clock, OutputSink, SinkError};

use document::{Diagnostic, Document};
use graph::{SiteGraph, Witness};
use library::{Corpus, DuplicatePageId};

/// What a build did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BuildReport {
    pub written: Vec<RelPath>,
    pub deleted: Vec<RelPath>,
    pub unchanged: usize,
    pub link_witnesses: Vec<Witness>,
    pub diagnostics: Vec<(library::PageId, Diagnostic)>,
}

/// Why a build stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteError {
    Sink(SinkError),
    /// `Inv_K`: a command named no page or output (plan §3.2).
    InvariantViolation(String),
}

/// Parses and resolves a corpus: the shared front half of build and check.
#[must_use]
pub fn analyse(corpus: &Corpus) -> (Vec<Document>, SiteGraph) {
    let documents: Vec<Document> = corpus.pages().map(document::parse).collect();
    let graph = graph::resolve(&documents);
    (documents, graph)
}

/// Every witness `rhawiki check` reports, in a stable order.
#[must_use]
pub fn witnesses(
    duplicates: &[DuplicatePageId],
    documents: &[Document],
    graph: &SiteGraph,
) -> Vec<CheckWitness> {
    let mut out: Vec<CheckWitness> = duplicates
        .iter()
        .map(|d| CheckWitness::DuplicatePageId {
            id: d.id.to_string(),
        })
        .collect();
    for doc in documents {
        for d in &doc.diagnostics {
            if let Diagnostic::DuplicateSlug { slug, .. } = d {
                out.push(CheckWitness::DuplicateSlug {
                    page: doc.id.to_string(),
                    slug: slug.clone(),
                });
            }
        }
    }
    for w in &graph.witnesses {
        out.push(match w {
            Witness::BrokenLink { from, target, .. } => CheckWitness::BrokenLink {
                from: from.to_string(),
                target: target.clone(),
            },
            Witness::AmbiguousLink { from, target, .. } => CheckWitness::AmbiguousLink {
                from: from.to_string(),
                target: target.clone(),
            },
            Witness::MissingAnchor {
                from,
                target,
                heading,
                ..
            } => CheckWitness::MissingAnchor {
                from: from.to_string(),
                target: target.clone(),
                heading: heading.clone(),
            },
        });
    }
    out.sort();
    out
}

/// A `rhawiki check` witness, in the registered corpus's key shape.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckWitness {
    DuplicatePageId {
        id: String,
    },
    DuplicateSlug {
        page: String,
        slug: String,
    },
    BrokenLink {
        from: String,
        target: String,
    },
    AmbiguousLink {
        from: String,
        target: String,
    },
    MissingAnchor {
        from: String,
        target: String,
        heading: String,
    },
}

/// Builds every page: observe the sink, parse, resolve, step, apply.
///
/// # Errors
/// A sink failure, or an `Inv_K` violation from the step.
pub fn build_all(
    corpus: &Corpus,
    clock: &impl Clock,
    sink: &mut impl OutputSink,
    renderer: &impl PageRenderer,
) -> Result<BuildReport, SiteError> {
    let observation = build::Observation {
        outputs: sink.list().map_err(SiteError::Sink)?.into_iter().collect(),
    };
    let (documents, graph) = analyse(corpus);
    let context = assembly::AssembleContext {
        built_at: Some(clock.now()),
    };
    let inputs = build::BuildInputs {
        documents: &documents,
        graph: &graph,
        context,
    };
    let (_, commands) = build::step(
        &build::SiteState::default(),
        &observation,
        &inputs,
        &assembly::DefaultAssembler,
        renderer,
    )
    .map_err(SiteError::InvariantViolation)?;
    let mut report = BuildReport {
        link_witnesses: graph.witnesses.clone(),
        ..BuildReport::default()
    };
    let produced = commands
        .iter()
        .filter(|c| matches!(c, build::Command::Write { .. }))
        .count();
    for command in commands {
        match command {
            build::Command::Write { path, bytes, .. } => {
                sink.write(&path, &bytes).map_err(SiteError::Sink)?;
                report.written.push(path);
            }
            build::Command::Delete { path } => {
                sink.delete(&path).map_err(SiteError::Sink)?;
                report.deleted.push(path);
            }
        }
    }
    report.unchanged = documents.len().saturating_sub(produced);
    report.diagnostics = documents
        .iter()
        .flat_map(|d| d.diagnostics.iter().map(|x| (d.id.clone(), x.clone())))
        .collect();
    Ok(report)
}
