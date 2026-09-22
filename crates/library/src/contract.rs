//! The `SourceRepository` contract suite (§9.8), run by every implementation.
//! It returns witnesses; an empty list means the implementation met the
//! contract on this input.

use std::collections::BTreeSet;

use crate::{RelPath, RepositoryError, SourceRepository};

/// A contract violation, with the evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    ListFailed(RepositoryError),
    DuplicateInListing(RelPath),
    ListingUnstable,
    ReadFailed {
        path: RelPath,
        error: RepositoryError,
    },
    ReadUnstable(RelPath),
    UnlistedReadable(RelPath),
    ListingIncomplete {
        missing: Vec<RelPath>,
    },
}

/// Checks `repository` against `expected`, the pages the fixture holds.
#[must_use]
pub fn source_repository(
    repository: &impl SourceRepository,
    expected: &[RelPath],
) -> Vec<Violation> {
    let mut out = Vec::new();
    let first = match repository.list() {
        Ok(list) => list,
        Err(e) => return vec![Violation::ListFailed(e)],
    };
    let mut seen = BTreeSet::new();
    for path in &first {
        if !seen.insert(path.clone()) {
            out.push(Violation::DuplicateInListing(path.clone()));
        }
    }
    match repository.list() {
        Ok(second) if second == first => {}
        _ => out.push(Violation::ListingUnstable),
    }
    let missing: Vec<RelPath> = expected
        .iter()
        .filter(|p| !seen.contains(*p))
        .cloned()
        .collect();
    if !missing.is_empty() {
        out.push(Violation::ListingIncomplete { missing });
    }
    for path in &seen {
        match (repository.read(path), repository.read(path)) {
            (Ok(a), Ok(b)) if a == b => {}
            (Ok(_), Ok(_)) => out.push(Violation::ReadUnstable(path.clone())),
            (Err(error), _) | (_, Err(error)) => out.push(Violation::ReadFailed {
                path: path.clone(),
                error,
            }),
        }
    }
    if let Ok(probe) = RelPath::new("__contract_probe__/absent.md")
        && !seen.contains(&probe)
        && repository.read(&probe).is_ok()
    {
        out.push(Violation::UnlistedReadable(probe));
    }
    out
}
