use std::collections::BTreeMap;

use crate::{PageId, RelPath, RepositoryError, Source, SourceRepository};

/// Two or more files with one page id after NFC (plan §3.2 witness).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicatePageId {
    pub id: PageId,
    /// Every path with that id, sorted.
    pub paths: Vec<RelPath>,
}

/// Loading failed; no corpus exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryError {
    Repository(RepositoryError),
}

/// The pages of a site, by id. Unique ids by construction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Corpus {
    pages: BTreeMap<PageId, Source>,
}

impl Corpus {
    /// A corpus of unique ids, or the duplicates.
    ///
    /// # Errors
    /// Every id that more than one source carries.
    pub fn new(sources: Vec<Source>) -> Result<Self, Vec<DuplicatePageId>> {
        let (corpus, duplicates) = Self::with_witnesses(sources);
        if duplicates.is_empty() {
            Ok(corpus)
        } else {
            Err(duplicates)
        }
    }

    /// A corpus keeping, for each id, the source with the smallest path, and
    /// one witness per duplicated id. `rhawiki check` uses this so that it
    /// can report every other witness too.
    #[must_use]
    pub fn with_witnesses(sources: Vec<Source>) -> (Self, Vec<DuplicatePageId>) {
        let mut by_id: BTreeMap<PageId, Vec<Source>> = BTreeMap::new();
        for source in sources {
            by_id.entry(source.id.clone()).or_default().push(source);
        }
        let mut pages = BTreeMap::new();
        let mut duplicates = Vec::new();
        for (id, mut group) in by_id {
            group.sort_by(|a, b| a.path.cmp(&b.path));
            if group.len() > 1 {
                duplicates.push(DuplicatePageId {
                    id: id.clone(),
                    paths: group.iter().map(|s| s.path.clone()).collect(),
                });
            }
            let first = group.swap_remove(0);
            pages.insert(id, first);
        }
        (Self { pages }, duplicates)
    }

    pub fn pages(&self) -> impl Iterator<Item = &Source> {
        self.pages.values()
    }

    #[must_use]
    pub fn get(&self, id: &PageId) -> Option<&Source> {
        self.pages.get(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.pages.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}

/// Reads every listed page. Complete or fail: one unreadable page fails the
/// whole load, and no corpus is returned. Duplicate ids are not an error
/// here; they are witnesses of [`Corpus::with_witnesses`].
///
/// # Errors
/// The first repository error.
pub fn load(
    repository: &impl SourceRepository,
) -> Result<(Corpus, Vec<DuplicatePageId>), LibraryError> {
    let mut paths = repository.list().map_err(LibraryError::Repository)?;
    paths.retain(RelPath::is_page);
    paths.sort();
    paths.dedup();
    let mut sources = Vec::with_capacity(paths.len());
    for path in paths {
        let text = repository.read(&path).map_err(LibraryError::Repository)?;
        sources.push(Source::new(path, text));
    }
    Ok(Corpus::with_witnesses(sources))
}
