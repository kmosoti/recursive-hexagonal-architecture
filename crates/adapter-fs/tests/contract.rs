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
        (RelPath::new("a.md").expect("v"), "alpha".to_owned()),
        (RelPath::new("b/c.md").expect("v"), "gamma".to_owned()),
    ];
    let repo = FsSources::new(&root);
    assert_eq!(
        library::contract::source_repository(&repo, &expected),
        vec![]
    );
    let paths: Vec<RelPath> = expected.iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(
        library::SourceRepository::list(&repo).expect("list"),
        paths,
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

#[cfg(unix)]
#[test]
fn nothing_is_written_through_a_symlinked_directory_or_a_planted_temporary() {
    use site::OutputSink as _;
    let out = temp("escape-out");
    let elsewhere = temp("escape-target");
    std::os::unix::fs::symlink(&elsewhere, out.join("guide")).expect("symlink");
    let mut sink = FsSink::new(&out);
    let err = sink.write(&RelPath::new("guide/page.html").expect("v"), b"x");
    assert!(
        err.is_err(),
        "writing through a symlinked directory is refused"
    );
    assert!(
        !elsewhere.join("page.html").exists(),
        "nothing escaped the output root"
    );
    // A page whose name looks like a temporary file is an ordinary output.
    sink.write(&RelPath::new(".rhawiki-tmp-foo.html").expect("v"), b"page")
        .expect("write");
    sink.write(&RelPath::new("foo.html").expect("v"), b"other")
        .expect("write");
    let listed: Vec<String> = sink
        .list()
        .expect("list")
        .into_iter()
        .map(|(p, _)| p.to_string())
        .collect();
    assert!(
        listed.contains(&".rhawiki-tmp-foo.html".to_owned())
            && listed.contains(&"foo.html".to_owned()),
        "{listed:?}"
    );
    std::fs::remove_dir_all(out).expect("clean");
    std::fs::remove_dir_all(elsewhere).expect("clean");
}

#[cfg(unix)]
#[test]
fn a_symlinked_output_root_is_refused_before_listing_writing_or_deleting() {
    use site::OutputSink as _;
    let parent = temp("root-link");
    let populated = temp("root-link-target");
    std::fs::write(populated.join("keep.txt"), b"not ours").expect("seed");
    let link = parent.join("out");
    std::os::unix::fs::symlink(&populated, &link).expect("symlink");
    let page = RelPath::new("keep.txt").expect("v");
    // Each spelling names the link itself; a trailing `/` or `/.` must not
    // resolve through it.
    let spellings = [
        link.clone(),
        std::path::PathBuf::from(format!("{}/", link.display())),
        std::path::PathBuf::from(format!("{}/.", link.display())),
    ];
    for spelling in spellings {
        let mut sink = FsSink::new(&spelling);
        let at = spelling.display();
        assert!(sink.list().is_err(), "{at}: the target is not inventoried");
        assert!(sink.write(&page, b"x").is_err(), "{at}: nothing is written");
        assert!(sink.delete(&page).is_err(), "{at}: nothing is deleted");
    }
    assert_eq!(
        std::fs::read(populated.join("keep.txt")).expect("still there"),
        b"not ours"
    );
}
