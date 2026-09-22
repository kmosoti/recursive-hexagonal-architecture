//! In-process fakes, always compiled (plan §3.1): pure, no dependencies.

use std::collections::BTreeMap;

use crate::{RelPath, RepositoryError, SourceRepository};

/// A repository over an in-memory map.
#[derive(Debug, Clone, Default)]
pub struct MemorySources {
    files: BTreeMap<RelPath, String>,
}

impl MemorySources {
    /// # Panics
    /// If a path is not a valid [`RelPath`]; a fake's input is test data.
    #[must_use]
    pub fn new(files: &[(&str, &str)]) -> Self {
        let mut map = BTreeMap::new();
        for (path, text) in files {
            #[allow(clippy::expect_used)]
            let path = RelPath::new(path).expect("fake paths are valid");
            map.insert(path, (*text).to_owned());
        }
        Self { files: map }
    }
}

impl SourceRepository for MemorySources {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        Ok(self.files.keys().cloned().collect())
    }

    fn read(&self, path: &RelPath) -> Result<String, RepositoryError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| RepositoryError::NotFound(path.clone()))
    }
}
