//! The registered P-B corpora are pinned by their tree digests: an edit to a
//! fixture or an expectation after registration fails here.

use std::path::Path;

fn tree_digest(root: &Path) -> String {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .expect("dir")
            .map(|e| e.expect("entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                walk(&path, base, out);
            } else if name != "registration.toml" && name != "GENERATOR-PROMPT.md" {
                let rel = path
                    .strip_prefix(base)
                    .expect("rel")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(format!(
                    "{rel} {}\n",
                    xtask::util::sha256_hex(&std::fs::read(&path).expect("read"))
                ));
            }
        }
    }
    let mut lines = Vec::new();
    walk(root, root, &mut lines);
    lines.sort();
    xtask::util::sha256_hex(lines.concat().as_bytes())
}

fn pinned(rel: &str) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    let reg: toml::Value = toml::from_str(
        &std::fs::read_to_string(base.join("registration.toml")).expect("registration"),
    )
    .expect("toml");
    assert_eq!(
        tree_digest(&base),
        reg["tree_sha256"].as_str().expect("digest"),
        "{rel} changed after registration"
    );
}

#[test]
fn the_verifier_corpus_is_exactly_what_was_registered() {
    pinned("tools/rha-verifier/tests/corpus");
}

#[test]
fn the_record_corpus_is_exactly_what_was_registered() {
    pinned("xtask/tests/corpus/records");
}
