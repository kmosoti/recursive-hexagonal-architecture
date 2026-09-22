//! The `corpus` command refuses a directory that is not the registered
//! corpus, so a wrong path or deleted fixtures cannot yield a `passed` H5
//! record (PR 17, automated review thread).

use std::path::{Path, PathBuf};
use std::process::Command;

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

fn temp(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rha-verifier-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp");
    dir
}

fn run(dir: &Path, out: &Path) -> (Option<i32>, usize) {
    let status = Command::new(env!("CARGO_BIN_EXE_rha-verifier"))
        .args(["corpus"])
        .arg(dir)
        .arg(out)
        .status()
        .expect("spawn");
    let records = std::fs::read_dir(out).map_or(0, Iterator::count);
    (status.code(), records)
}

#[test]
fn an_empty_directory_is_refused_and_writes_no_record() {
    let dir = temp("empty");
    let out = temp("empty-out");
    assert_eq!(run(&dir, &out), (Some(2), 0));
}

#[test]
fn a_corpus_with_a_deleted_fixture_is_refused() {
    let dir = temp("partial");
    let out = temp("partial-out");
    let mut deleted = false;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(corpus())
        .expect("corpus")
        .map(|e| e.expect("entry").path())
        .collect();
    entries.sort();
    for path in entries.iter().filter(|p| p.is_file()) {
        let is_fixture = path.extension().is_some_and(|e| e == "json");
        if is_fixture && !deleted {
            deleted = true;
            continue;
        }
        std::fs::copy(path, dir.join(path.file_name().expect("name"))).expect("copy");
    }
    assert!(deleted);
    assert_eq!(run(&dir, &out), (Some(2), 0));
}

#[test]
fn the_registered_corpus_is_decided_as_registered() {
    let out = temp("full-out");
    assert_eq!(run(&corpus(), &out), (Some(0), 1));
}
