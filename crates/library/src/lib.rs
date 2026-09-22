//! `library`: page identity, sources, and complete-or-fail loading (BDR-0001).
//!
//! A page is a `.md` file under a root. Its [`PageId`] is its relative path
//! without the suffix, `/`-separated, NFC-normalized, case preserved. Loading
//! goes through the [`SourceRepository`] port and either yields every listed
//! page or fails; there is no partial corpus.
#![forbid(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::disallowed_macros
)]

pub mod contract;
mod corpus;
mod digest;
mod id;
mod port;
pub mod testing;

pub use corpus::{Corpus, DuplicatePageId, LibraryError, load};
pub use digest::Digest;
pub use id::{PageId, RelPath, RelPathError};
pub use port::{RepositoryError, SourceRepository};

/// One page's text with its identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: PageId,
    pub path: RelPath,
    pub text: String,
    pub digest: Digest,
}

impl Source {
    /// A source whose id and digest are derived from its path and text.
    #[must_use]
    pub fn new(path: RelPath, text: String) -> Self {
        Self {
            id: PageId::from_path(&path),
            digest: Digest::of(text.as_bytes()),
            path,
            text,
        }
    }
}
