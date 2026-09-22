//! The composition root end to end: the registered markdown corpus (P-A stage
//! 4, graded exactly as registered) and the W5 acceptance on this repository's
//! own docs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn check(dir: &Path) -> (Option<i32>, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_rhawiki"))
        .args(["check", "--format", "json", "--root"])
        .arg(dir)
        .output()
        .expect("rhawiki runs");
    (
        out.status.code(),
        serde_json::from_slice(&out.stdout).expect("json"),
    )
}

fn multiset(v: &serde_json::Value) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for w in v.as_array().expect("array") {
        *m.entry(serde_json::to_string(w).expect("json"))
            .or_default() += 1;
    }
    m
}

/// Every registered site: exactly the expected witnesses, as a multiset with
/// every key equal and no extras, and exit 1 exactly when there are any.
#[test]
fn the_registered_markdown_corpus_passes_as_registered() {
    let sites = root().join("xtask/tests/corpus/markdown/sites");
    let mut failures = Vec::new();
    let mut count = 0;
    let mut dirs: Vec<_> = std::fs::read_dir(&sites)
        .expect("sites")
        .map(|e| e.expect("entry").path())
        .collect();
    dirs.sort();
    for site in dirs {
        count += 1;
        let expected: serde_json::Value =
            serde_json::from_slice(&std::fs::read(site.join("EXPECTED.json")).expect("expected"))
                .expect("json");
        // Normalize key order by round-tripping through serde_json::Value.
        let want = multiset(&expected["witnesses"]);
        let (code, got) = check(&site);
        let has = !want.is_empty();
        if multiset(&got["witnesses"]) != want || code != Some(i32::from(has)) {
            failures.push(
                site.file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    assert_eq!(count, 60, "the registration has 60 sites");
    assert!(
        failures.is_empty(),
        "sites not graded as registered: {failures:?}"
    );
}

#[test]
fn this_repository_has_no_witnesses() {
    let (code, got) = check(&root().join("docs"));
    assert_eq!(code, Some(0), "{got}");
    assert_eq!(got["witnesses"], serde_json::json!([]));
}

#[test]
fn the_spec_renders_with_every_heading_and_both_mermaid_blocks() {
    let out = std::env::temp_dir().join(format!("rhawiki-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(env!("CARGO_BIN_EXE_rhawiki"))
        .args(["build", "--root"])
        .arg(root().join("docs"))
        .arg("--out")
        .arg(&out)
        .status()
        .expect("rhawiki builds");
    assert!(status.success());
    let spec = std::fs::read_to_string(out.join("spec/rha-spec-v0.10.html")).expect("spec page");
    let headings = spec
        .lines()
        .filter(|l| l.starts_with("<h") && l.contains(" id=\""))
        .count();
    assert_eq!(headings, 182, "plan W5: 182 headings outside code fences");
    assert_eq!(spec.matches("<pre class=\"mermaid\">").count(), 2);
    assert!(out.join("assets/style.css").is_file());
    std::fs::remove_dir_all(out).expect("clean");
}
