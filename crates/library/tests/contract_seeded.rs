//! Seeded violators (plan W5 brief): each must be reported by the suite, and
//! the honest fake must pass it.

use std::cell::Cell;

use library::contract::{Violation, source_repository};
use library::testing::MemorySources;
use library::{RelPath, RepositoryError, SourceRepository};

const FILES: [(&str, &str); 3] = [
    ("a.md", "alpha text"),
    ("b/c.md", "gamma"),
    ("d.md", "delta"),
];

fn expected() -> Vec<(RelPath, String)> {
    FILES
        .iter()
        .map(|(p, t)| (RelPath::new(p).expect("valid"), (*t).to_owned()))
        .collect()
}

#[test]
fn the_memory_fake_meets_the_contract() {
    let repo = MemorySources::new(&FILES);
    assert_eq!(source_repository(&repo, &expected()), vec![]);
}

/// Drops the last file from the listing.
struct SilentPartialSource(MemorySources);
impl SourceRepository for SilentPartialSource {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        let mut all = self.0.list()?;
        all.pop();
        Ok(all)
    }
    fn read(&self, p: &RelPath) -> Result<String, RepositoryError> {
        self.0.read(p)
    }
}

/// W5 brief: returns half the text, on every read (consistently).
struct TruncatingSource(MemorySources);
impl SourceRepository for TruncatingSource {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        self.0.list()
    }
    fn read(&self, p: &RelPath) -> Result<String, RepositoryError> {
        let text = self.0.read(p)?;
        Ok(text[..text.len() / 2].to_owned())
    }
}

/// Alternates whole and truncated reads.
struct UnstableReadSource(MemorySources, Cell<bool>);
impl SourceRepository for UnstableReadSource {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        self.0.list()
    }
    fn read(&self, p: &RelPath) -> Result<String, RepositoryError> {
        let text = self.0.read(p)?;
        let half = self.1.replace(!self.1.get());
        Ok(if half {
            text[..text.len() / 2].to_owned()
        } else {
            text
        })
    }
}

/// Changes the listing's order between calls.
struct UnstablePathsSource(MemorySources, Cell<u32>);
impl SourceRepository for UnstablePathsSource {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        self.1.set(self.1.get() + 1);
        let mut all = self.0.list()?;
        if self.1.get().is_multiple_of(2) {
            all.reverse();
        }
        Ok(all)
    }
    fn read(&self, p: &RelPath) -> Result<String, RepositoryError> {
        self.0.read(p)
    }
}

/// Lists one path twice.
struct DuplicatePathSource(MemorySources);
impl SourceRepository for DuplicatePathSource {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        let mut all = self.0.list()?;
        all.push(all[0].clone());
        Ok(all)
    }
    fn read(&self, p: &RelPath) -> Result<String, RepositoryError> {
        self.0.read(p)
    }
}

#[test]
fn every_seeded_violator_is_reported() {
    let expected = expected();
    let partial = source_repository(&SilentPartialSource(MemorySources::new(&FILES)), &expected);
    assert!(
        partial
            .iter()
            .any(|v| matches!(v, Violation::ListingIncomplete { .. })),
        "{partial:?}"
    );
    let truncating = source_repository(&TruncatingSource(MemorySources::new(&FILES)), &expected);
    assert!(
        truncating
            .iter()
            .any(|v| matches!(v, Violation::ContentMismatch(_))),
        "a consistently truncating source is caught: {truncating:?}"
    );
    let unstable_reads = source_repository(
        &UnstableReadSource(MemorySources::new(&FILES), Cell::new(false)),
        &expected,
    );
    assert!(
        unstable_reads
            .iter()
            .any(|v| matches!(v, Violation::ReadUnstable(_))),
        "{unstable_reads:?}"
    );
    let unstable = source_repository(
        &UnstablePathsSource(MemorySources::new(&FILES), Cell::new(0)),
        &expected,
    );
    assert!(
        unstable.contains(&Violation::ListingUnstable),
        "{unstable:?}"
    );
    let duplicate = source_repository(&DuplicatePathSource(MemorySources::new(&FILES)), &expected);
    assert!(
        duplicate
            .iter()
            .any(|v| matches!(v, Violation::DuplicateInListing(_))),
        "{duplicate:?}"
    );
}
