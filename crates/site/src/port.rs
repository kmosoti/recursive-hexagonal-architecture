use library::{Digest, RelPath};

/// Why the sink failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkError {
    pub path: Option<RelPath>,
    pub message: String,
}

/// Where rendered output goes (glue port).
///
/// Assumption `site.glue.sink_atomic_listable`: `write` is all-or-nothing and
/// `list` reflects every completed write with its digest.
pub trait OutputSink {
    /// # Errors
    /// When the listing cannot be produced.
    fn list(&self) -> Result<Vec<(RelPath, Digest)>, SinkError>;
    /// # Errors
    /// When the write did not complete.
    fn write(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), SinkError>;
    /// # Errors
    /// When the delete did not complete.
    fn delete(&mut self, path: &RelPath) -> Result<(), SinkError>;
}

/// The time shown in the footer (glue port). Display only: assumption
/// `site.glue.clock_display_only`.
pub trait Clock {
    fn now(&self) -> String;
}
