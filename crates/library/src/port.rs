use crate::RelPath;

/// Why the repository could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    NotFound(RelPath),
    NotUtf8(RelPath),
    Io {
        path: Option<RelPath>,
        message: String,
    },
}

/// Where page sources come from (the required port of `library`).
///
/// Assumption `library.source_repository.complete_or_fail`: `list` returns
/// every `.md` page under the root exactly once with stable relative paths,
/// and `read` returns the whole text or fails. `library::contract` checks it.
pub trait SourceRepository {
    /// Every page path.
    ///
    /// # Errors
    /// When the listing cannot be produced.
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError>;

    /// The whole text of one page.
    ///
    /// # Errors
    /// When the page is missing, not UTF-8, or unreadable.
    fn read(&self, path: &RelPath) -> Result<String, RepositoryError>;
}
