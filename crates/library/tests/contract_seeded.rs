//! Seeded violators (plan W5 brief): each must be reported by the suite, and
//! the honest fake must pass it.

use std::cell::Cell;

use library::contract::{Violation, source_repository};
use library::testing::MemorySources;
use library::{RelPath, RepositoryError, SourceRepository};

fn paths(names: &[&str]) -> Vec<RelPath> {
    names
        .iter()
        .map(|n| RelPath::new(n).expect("valid"))
        .collect()
}

const FILES: [(&str, &str); 3] = [
    ("a.md", "alpha text"),
    ("b/c.md", "gamma"),
    ("d.md", "delta"),
];

#[test]
fn the_memory_fake_meets_the_contract() {
    let repo = MemorySources::new(&FILES);
    assert_eq!(
        source_repository(&repo, &paths(&["a.md", "b/c.md", "d.md"])),
        vec![]
    );
}

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

struct TruncatingSource(MemorySources, Cell<bool>);
impl SourceRepository for TruncatingSource {
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
    let expected = paths(&["a.md", "b/c.md", "d.md"]);
    let partial = source_repository(&SilentPartialSource(MemorySources::new(&FILES)), &expected);
    assert!(
        partial
            .iter()
            .any(|v| matches!(v, Violation::ListingIncomplete { .. })),
        "{partial:?}"
    );
    let truncating = source_repository(
        &TruncatingSource(MemorySources::new(&FILES), Cell::new(false)),
        &expected,
    );
    assert!(
        truncating
            .iter()
            .any(|v| matches!(v, Violation::ReadUnstable(_))),
        "{truncating:?}"
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
