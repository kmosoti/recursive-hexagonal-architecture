//! `document`: a page's markdown as a typed document (BDR-0002).
//!
//! Parsing is total: every input yields a [`Document`], and anything the
//! model does not carry is a visible [`Diagnostic`]. No pulldown-cmark type
//! appears in this crate's public API (BDR-0002 criterion a).
#![forbid(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::disallowed_macros
)]

mod citation;
mod node;
mod parse;
mod section;
mod section_ref;
mod slug;

pub use citation::{Citation, ReferenceEntry};
pub use node::{Align, CalloutKind, Node, Transclusion};
pub use parse::parse;
pub use section::{section_nodes, transclusions};
pub use section_ref::SectionRef;
pub use slug::slugify;

use library::{Digest, PageId};

/// A parsed page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: PageId,
    pub source_digest: Digest,
    /// The first level-1 heading's text, else the id's basename.
    pub title: String,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub body: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
    pub section_refs: Vec<SectionRef>,
    pub reference_entries: Vec<ReferenceEntry>,
    pub citations: Vec<Citation>,
}

/// A heading with its final, unique slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// Unique within the page: a repeated base slug gets `-2`, `-3`, ….
    pub slug: String,
    /// The slug before de-duplication.
    pub base_slug: String,
    pub line: usize,
}

/// A wikilink as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The target as written; empty for `[[#heading]]`.
    pub target: String,
    pub anchor: Option<String>,
    pub alias: Option<String>,
    pub line: usize,
}

/// What parsing observed that a reader should see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    /// A heading's base slug repeats an earlier one on the page.
    DuplicateSlug {
        slug: String,
        first_line: usize,
        second_line: usize,
    },
    /// An unresolved section reference on a page with at least one numbered heading.
    UnresolvedSectionRef { number: String, line: usize },
    /// A reference entry's label duplicates an earlier one on the page.
    DuplicateReferenceEntry {
        label: String,
        first_line: usize,
        second_line: usize,
    },
    /// An unresolved citation on a page with at least one reference entry.
    UnresolvedCitation { label: String, line: usize },
    /// Syntax the model preserves only as text.
    Unsupported { kind: &'static str, line: usize },
}
