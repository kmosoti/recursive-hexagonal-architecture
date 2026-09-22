//! Plan §3.2: ids unique or witnessed; complete-or-fail loading.

use library::testing::MemorySources;
use library::{Corpus, LibraryError, RelPath, RepositoryError, Source, SourceRepository, load};
use proptest::prelude::*;

fn name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "a",
        "A",
        "b",
        "café",
        "cafe\u{301}",
        "x/y",
        "x/Y",
        "\u{65e5}\u{672c}",
    ])
    .prop_map(str::to_owned)
}

proptest! {
    #[test]
    fn ids_are_unique_or_every_duplicate_is_witnessed(names in prop::collection::vec(name(), 0..8)) {
        let sources: Vec<Source> = names
            .iter()
            .enumerate()
            .map(|(i, n)| Source::new(RelPath::new(&format!("{n}.md")).expect("valid"), format!("page {i}")))
            .collect();
        let (corpus, dups) = Corpus::with_witnesses(sources.clone());
        let mut ids: Vec<_> = sources.iter().map(|s| s.id.clone()).collect();
        ids.sort();
        ids.dedup();
        prop_assert_eq!(corpus.len(), ids.len());
        for id in &ids {
            let count = sources.iter().filter(|s| &s.id == id).count();
            let witnessed = dups.iter().any(|d| &d.id == id && d.paths.len() == count);
            prop_assert_eq!(count > 1, witnessed);
        }
    }
}

struct FailsOn(MemorySources, &'static str);
impl SourceRepository for FailsOn {
    fn list(&self) -> Result<Vec<RelPath>, RepositoryError> {
        self.0.list()
    }
    fn read(&self, p: &RelPath) -> Result<String, RepositoryError> {
        if p.as_str() == self.1 {
            Err(RepositoryError::Io {
                path: Some(p.clone()),
                message: "seeded".to_owned(),
            })
        } else {
            self.0.read(p)
        }
    }
}

#[test]
fn one_unreadable_page_fails_the_whole_load() {
    let repo = FailsOn(MemorySources::new(&[("a.md", "x"), ("b.md", "y")]), "b.md");
    assert!(matches!(load(&repo), Err(LibraryError::Repository(_))));
    let ok = MemorySources::new(&[("a.md", "x"), ("b.md", "y"), ("notes.txt", "z")]);
    let (corpus, dups) = load(&ok).expect("loads");
    assert_eq!(corpus.len(), 2, "only .md files are pages");
    assert!(dups.is_empty());
}
