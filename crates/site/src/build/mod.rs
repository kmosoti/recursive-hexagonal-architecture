//! `site::build`: the pure step, `(state, observation) -> (state', commands)`.
//! Uses its sibling `assembly` (declared in rha-modules.toml) and nothing of
//! the glue.

use std::collections::{BTreeMap, BTreeSet};

use document::Document;
use graph::SiteGraph;
use library::{Digest, PageId, RelPath};

use crate::assembly::{Assemble, AssembleContext, PageModel};

/// A rendered file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub path: RelPath,
    pub bytes: Vec<u8>,
}

/// The rendering port, exported at the crate root (plan §3.1).
///
/// Assumption `site.build.renderer_deterministic_total`: never panics, and
/// equal models give equal bytes. `site::contract::page_renderer` checks it.
pub trait PageRenderer {
    fn render(&self, page: &PageModel) -> Rendered;
    /// Files every build writes besides pages (for example a stylesheet).
    fn assets(&self) -> Vec<Rendered>;
}

/// What the sink held before the step.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Observation {
    pub outputs: BTreeMap<RelPath, Digest>,
}

/// What the step believes the output holds afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SiteState {
    pub outputs: BTreeMap<RelPath, Digest>,
}

/// Inputs of one step.
pub struct BuildInputs<'a> {
    pub documents: &'a [Document],
    pub graph: &'a SiteGraph,
    pub context: AssembleContext,
}

/// An effect for the glue to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `page` is `None` for an asset.
    Write {
        path: RelPath,
        bytes: Vec<u8>,
        page: Option<PageId>,
    },
    Delete {
        path: RelPath,
    },
}

/// The step. Writes what differs from the observation, deletes what is no
/// longer produced, and checks `Inv_K` before returning.
///
/// # Errors
/// An `Inv_K` witness: a write naming no input page or asset, a delete naming
/// no observed output, or two writes to one path.
pub fn step(
    _state: &SiteState,
    observation: &Observation,
    inputs: &BuildInputs<'_>,
    assembler: &impl Assemble,
    renderer: &impl PageRenderer,
) -> Result<(SiteState, Vec<Command>), String> {
    let titles: BTreeMap<&PageId, &str> = inputs
        .documents
        .iter()
        .map(|d| (&d.id, d.title.as_str()))
        .collect();
    let lookup = |id: &PageId| titles.get(id).map(|t| (*t).to_owned());
    let mut produced: BTreeMap<RelPath, (Vec<u8>, Option<PageId>)> = BTreeMap::new();
    let mut commands = Vec::new();
    for doc in inputs.documents {
        let model = assembler.assemble(doc, inputs.graph, &lookup, &inputs.context);
        let out = renderer.render(&model);
        if produced
            .insert(out.path.clone(), (out.bytes, Some(doc.id.clone())))
            .is_some()
        {
            return Err(format!("Inv_K: two writes to {}", out.path));
        }
    }
    for asset in renderer.assets() {
        if produced
            .insert(asset.path.clone(), (asset.bytes, None))
            .is_some()
        {
            return Err(format!("Inv_K: two writes to {}", asset.path));
        }
    }
    let mut state = SiteState::default();
    for (path, (bytes, page)) in &produced {
        let digest = Digest::of(bytes);
        state.outputs.insert(path.clone(), digest);
        if observation.outputs.get(path) != Some(&digest) {
            commands.push(Command::Write {
                path: path.clone(),
                bytes: bytes.clone(),
                page: page.clone(),
            });
        }
    }
    for path in observation.outputs.keys() {
        if !produced.contains_key(path) {
            commands.push(Command::Delete { path: path.clone() });
        }
    }
    check_inv_k(&commands, observation, inputs)?;
    Ok((state, commands))
}

/// `Inv_K` (plan §3.2): every write names an input page or is an asset; every
/// delete names an observed output. The check does not use the step's code.
///
/// # Errors
/// The first violating command, with the observation's size.
pub fn check_inv_k(
    commands: &[Command],
    observation: &Observation,
    inputs: &BuildInputs<'_>,
) -> Result<(), String> {
    let pages: BTreeSet<&PageId> = inputs.documents.iter().map(|d| &d.id).collect();
    for c in commands {
        match c {
            Command::Write {
                page: Some(p),
                path,
                ..
            } if !pages.contains(p) => {
                return Err(format!(
                    "Inv_K: write {path} names page {p}, which is not an input"
                ));
            }
            Command::Delete { path } if !observation.outputs.contains_key(path) => {
                return Err(format!(
                    "Inv_K: delete {path} is not in the observation of {} outputs",
                    observation.outputs.len()
                ));
            }
            _ => {}
        }
    }
    Ok(())
}
