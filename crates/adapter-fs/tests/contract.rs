//! adapter-fs runs its owners' contract suites (plan §3.1).

use adapter_fs::{FsSink, FsSources};
use library::RelPath;

fn temp(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rhawiki-fs-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp");
    dir
}

#[test]
fn fs_sources_meet_the_source_repository_contract() {
    let root = temp("src");
    std::fs::create_dir_all(root.join("b")).expect("dir");
    std::fs::write(root.join("a.md"), "alpha").expect("write");
    std::fs::write(root.join("b/c.md"), "gamma").expect("write");
    std::fs::write(root.join("notes.txt"), "not a page").expect("write");
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("a.md"), root.join("link.md")).expect("symlink");
    let expected = vec![
        RelPath::new("a.md").expect("v"),
        RelPath::new("b/c.md").expect("v"),
    ];
    let repo = FsSources::new(&root);
    assert_eq!(
        library::contract::source_repository(&repo, &expected),
        vec![]
    );
    assert_eq!(
        library::SourceRepository::list(&repo).expect("list"),
        expected,
        "no .txt, no symlink"
    );
    std::fs::remove_dir_all(root).expect("clean");
}

#[test]
fn fs_sink_meets_the_output_sink_contract() {
    let root = temp("sink");
    let mut sink = FsSink::new(&root);
    assert!(site::contract::output_sink(&mut sink).is_empty());
    std::fs::remove_dir_all(root).expect("clean");
}
