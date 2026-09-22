//! The registered adversarial markdown corpus (P-A stage 1) is pinned by its
//! tree digest: any edit to a site or an expectation fails here. Grading the
//! product against it happens in P-A stage 4, through `rhawiki check`.

use std::path::Path;

fn tree_digest(root: &Path) -> String {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .expect("read dir")
            .map(|e| e.expect("entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, base, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/");
                let digest = xtask::util::sha256_hex(&std::fs::read(&path).expect("read"));
                out.push(format!("{rel} {digest}\n"));
            }
        }
    }
    let mut lines = Vec::new();
    walk(root, root, &mut lines);
    lines.sort();
    xtask::util::sha256_hex(lines.concat().as_bytes())
}

#[test]
fn the_markdown_corpus_is_exactly_what_was_registered() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/markdown");
    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(base.join("registration.toml")).expect("registration"),
    )
    .expect("toml");
    assert_eq!(
        tree_digest(&base.join("sites")),
        registration["tree_sha256"].as_str().expect("digest"),
        "a registered site or expectation changed after registration"
    );
    let sites = std::fs::read_dir(base.join("sites"))
        .expect("sites")
        .count();
    assert_eq!(
        i64::try_from(sites).expect("count"),
        registration["sites"].as_integer().expect("sites")
    );
}
