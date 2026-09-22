//! `site::assembly`: a document and the site graph become a page model.
//! Pure. Uses no sibling and nothing of the glue (rha-modules.toml).

use document::{Document, Node};
use graph::SiteGraph;
use library::PageId;

/// Inputs every page shares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssembleContext {
    /// The footer's time, from the glue's clock; display only.
    pub built_at: Option<String>,
}

/// One entry of the table of contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub level: u8,
    pub text: String,
    pub anchor: String,
}

/// Where wikilink `i` of the page goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Page { id: PageId, anchor: Option<String> },
    Unresolved,
}

/// Everything a renderer needs for one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageModel {
    pub id: PageId,
    pub title: String,
    pub toc: Vec<TocEntry>,
    /// `(label, id of the index page if it exists)` for each parent segment.
    pub breadcrumbs: Vec<String>,
    pub backlinks: Vec<(PageId, String)>,
    pub links: Vec<LinkTarget>,
    pub body: Vec<Node>,
    pub built_at: Option<String>,
}

/// The assembly port (provided here, required by `build`).
///
/// Assumption `site.build.assemble_deterministic`: equal inputs give equal
/// models.
pub trait Assemble {
    fn assemble(
        &self,
        doc: &Document,
        graph: &SiteGraph,
        titles: &dyn Fn(&PageId) -> Option<String>,
        ctx: &AssembleContext,
    ) -> PageModel;
}

/// The default assembly.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultAssembler;

impl Assemble for DefaultAssembler {
    fn assemble(
        &self,
        doc: &Document,
        graph: &SiteGraph,
        titles: &dyn Fn(&PageId) -> Option<String>,
        ctx: &AssembleContext,
    ) -> PageModel {
        let toc = doc
            .headings
            .iter()
            .map(|h| TocEntry {
                level: h.level,
                text: h.text.clone(),
                anchor: h.slug.clone(),
            })
            .collect();
        let links = (0..doc.links.len())
            .map(|i| match graph.resolution(&doc.id, i) {
                Some(r) => LinkTarget::Page {
                    id: r.to.clone(),
                    anchor: r.anchor.clone(),
                },
                None => LinkTarget::Unresolved,
            })
            .collect();
        let backlinks = graph
            .backlinks
            .get(&doc.id)
            .map(|set| {
                set.iter()
                    .filter(|p| **p != doc.id)
                    .map(|p| (p.clone(), titles(p).unwrap_or_else(|| p.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let segments: Vec<&str> = doc.id.as_str().split('/').collect();
        let breadcrumbs = segments[..segments.len().saturating_sub(1)]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        PageModel {
            id: doc.id.clone(),
            title: doc.title.clone(),
            toc,
            breadcrumbs,
            backlinks,
            links,
            body: doc.body.clone(),
            built_at: ctx.built_at.clone(),
        }
    }
}
