//! End-to-end grader for CHG-011 (citations, `[Rn]`), written before any
//! implementation exists. It builds a small, standalone docs tree (not
//! part of the registered `xtask/tests/corpus/*` convention: this is a
//! purpose-built fixture for this one feature, following the pattern of
//! `crates/app-cli/tests/transclusion.rs`'s `write_sources`/`run_build`/
//! `run_check` helpers) and exercises `rhawiki build` and `rhawiki check`
//! through the CLI binary.
//!
//! `app-cli` is not a core crate, so this file uses `std::fs` freely
//! (AGENTS.md: "Core crates perform no I/O ... effects cross ports";
//! adapters, including the CLI, are where I/O happens).
//!
//! This grader was NOT stub-compiled: doing so would require standing up
//! throwaway stubs of `graph`, `site`, the HTML and JSON renderer adapters,
//! and `app-cli`'s own binary target, which is out of proportion to a
//! single feature's oracle. Only `crates/document/tests/citations.rs` (this
//! package's core grader) was stub-compiled and run to green against a
//! scratch stub of the proposed `document` API; see the oracle author's
//! report. This file's shapes (the JSON renderer's page envelope, the
//! `rhawiki check --format json` envelope, and the harness helpers below)
//! are copied from `crates/app-cli/tests/transclusion.rs` and from
//! `xtask/tests/corpus/transclusion/sites/CASES.json`'s registered
//! `expected_check`/`expected_pages` shapes, both read for API shape only.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

fn temp_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rha-citations-cli-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}

struct OwnedTempDir(PathBuf);

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read_file(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn write_page(root: &Path, relative_stem: &str, text: &str) {
    let path = root.join(format!("{relative_stem}.md"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn run_build(source: &Path, output: &Path, format: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rhawiki"))
        .args(["build", "--root"])
        .arg(source)
        .args(["--out"])
        .arg(output)
        .args(["--format", format])
        .output()
        .expect("rhawiki build must start")
}

fn run_check(source: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rhawiki"))
        .args(["check", "--root"])
        .arg(source)
        .args(["--format", "json"])
        .output()
        .expect("rhawiki check must start")
}

/// The docs tree this grader builds:
///
/// - `refs.md`: two reference entries (`R1`, `R2`), a resolved range
///   `[R1]-[R2]`, and an unresolved citation `[R9]` (contract section 1-2).
/// - `host.md`: transcludes `refs.md`'s `Bibliography` section, which
///   contains both entries (contract section 4: anchors are dropped from
///   transcluded content, so the host page never has two elements with one
///   id because of transclusion).
const REFS_MARKDOWN: &str = "\
# References Page

## Bibliography

[R1] Author One. (2020). Title One.

[R2] Author Two. (2021). Title Two.

## Discussion

See [R1] and [R2], the range [R1]-[R2], and an unresolved citation [R9].
";

const HOST_MARKDOWN: &str = "\
# Host Page

before transclusion

![[refs#bibliography]]

after transclusion
";

fn build_tree(source: &Path) {
    write_page(source, "refs", REFS_MARKDOWN);
    write_page(source, "host", HOST_MARKDOWN);
}

#[test]
fn citations_render_anchors_links_and_json_anchor_nodes() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    let json_output = temp.join("json");
    fs::create_dir(&source).unwrap();
    build_tree(&source);

    // --- HTML build: anchors, resolved-citation links, and the dropped
    // transcluded anchor (contract section 4). ---
    let html = run_build(&source, &html_output, "html");
    assert!(
        html.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&html.stderr)
    );

    let refs_html =
        String::from_utf8(read_file(&html_output.join("refs.html"))).expect("refs.html UTF-8");
    let host_html =
        String::from_utf8(read_file(&html_output.join("host.html"))).expect("host.html UTF-8");

    assert!(
        refs_html.contains("<a id=\"ref-r1\"></a>"),
        "refs.html: missing entry anchor ref-r1: {refs_html}"
    );
    assert!(
        refs_html.contains("<a id=\"ref-r2\"></a>"),
        "refs.html: missing entry anchor ref-r2"
    );
    assert!(
        refs_html.contains("<a href=\"#ref-r1\">[R1]</a>"),
        "refs.html: a citation must link to its entry's anchor with unchanged link text"
    );
    assert!(
        refs_html.contains("<a href=\"#ref-r2\">[R2]</a>"),
        "refs.html: a citation must link to its entry's anchor with unchanged link text"
    );
    // The range `[R1]-[R72]`-shaped `[R1]-[R2]` is two independently
    // resolved citations with the literal text `-` between them (contract
    // section 2): both link, and the `-` is not swallowed into either.
    assert!(
        refs_html.contains("<a href=\"#ref-r1\">[R1]</a>-<a href=\"#ref-r2\">[R2]</a>")
            || refs_html.contains("<a href=\"#ref-r1\">[R1]</a>-<a href=\"#ref-r2\">[R2]</a>,"),
        "refs.html: range citations [R1]-[R2] must render as two independent links joined by '-'"
    );
    // `[R9]` has no entry, so it stays text, unchanged.
    assert!(
        refs_html.contains("[R9]") && !refs_html.contains("href=\"#ref-r9\""),
        "refs.html: an unresolved citation must stay text, never a link"
    );

    // The transcluding host page copies the entry text but drops the
    // anchor: it must not carry `id="ref-r1"` or `id="ref-r2"` at all, so
    // it can never have two elements sharing one id because of
    // transclusion (contract section 4, decision `citations-scope`).
    assert!(
        !host_html.contains("id=\"ref-r1\""),
        "host.html: a transcluded entry's anchor must be dropped, not duplicated: {host_html}"
    );
    assert!(
        !host_html.contains("id=\"ref-r2\""),
        "host.html: a transcluded entry's anchor must be dropped, not duplicated"
    );
    assert!(
        host_html.contains("Author One") && host_html.contains("Author Two"),
        "host.html: the transcluded entry text itself must still be present"
    );

    // --- JSON build: {"type":"anchor","id":...} nodes, and no search text. ---
    let json_build = run_build(&source, &json_output, "json");
    assert!(
        json_build.status.success(),
        "JSON build failed: {}",
        String::from_utf8_lossy(&json_build.stderr)
    );

    let refs_json: Value =
        serde_json::from_slice(&read_file(&json_output.join("refs.json"))).expect("refs.json");
    let anchors = collect_anchor_nodes(&refs_json["body"]);
    let anchor_ids: BTreeSet<String> = anchors
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        anchor_ids,
        BTreeSet::from(["ref-r1".to_owned(), "ref-r2".to_owned()]),
        "refs.json: exactly one anchor node per reference entry, by id"
    );
    for anchor in &anchors {
        assert_eq!(
            anchor.as_object().unwrap().len(),
            2,
            "refs.json: an anchor node has exactly {{type, id}}, no extra members: {anchor}"
        );
    }

    let search_index: Value = serde_json::from_slice(&read_file(
        &json_output.join("assets").join("search-index.json"),
    ))
    .expect("search-index.json");
    let refs_entry = find_search_entry(&search_index, "refs");
    let indexed_text = refs_entry["text"]
        .as_str()
        .unwrap_or_else(|| panic!("refs search-index entry has no text field: {refs_entry}"));
    assert!(
        !indexed_text.contains("ref-r1") && !indexed_text.contains("ref-r2"),
        "search index text must not contain an anchor's id: an anchor adds no text: {indexed_text}"
    );
    assert!(
        indexed_text.contains("Author One") && indexed_text.contains("[R1]"),
        "search index text must still contain the entry's and citation's own visible text"
    );

    // --- `rhawiki check --format json`: unchanged witness surface
    // (decision `citations-check-surface`: no citation witnesses). ---
    let check = run_check(&source);
    assert!(
        check.status.success(),
        "check failed unexpectedly on a tree with no broken links: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let check_json: Value = serde_json::from_slice(&check.stdout).expect("check JSON");
    assert_eq!(check_json["schema_version"], 1, "check schema_version");
    assert_eq!(check_json["pages"], 2, "check page count");
    assert_eq!(
        check_json["witnesses"],
        Value::Array(vec![]),
        "a tree with no broken links, ambiguous links, missing anchors or transclusion \
         problems must report zero witnesses; in particular, neither an unresolved \
         citation nor a duplicate reference entry is a check witness (decision \
         citations-check-surface): {check_json}"
    );
    assert_eq!(
        check_json["counts"],
        serde_json::json!({}),
        "check counts must be empty: no witness kind (existing or new) fired"
    );
}

fn collect_anchor_nodes(nodes: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    collect_anchor_nodes_into(nodes, &mut out);
    out
}

fn collect_anchor_nodes_into(nodes: &Value, out: &mut Vec<Value>) {
    let Some(array) = nodes.as_array() else {
        return;
    };
    for node in array {
        if node["type"] == "anchor" {
            out.push(node.clone());
            continue;
        }
        for key in ["children"] {
            if node.get(key).is_some() {
                collect_anchor_nodes_into(&node[key], out);
            }
        }
        if let Some(items) = node.get("items").and_then(Value::as_array) {
            for item in items {
                collect_anchor_nodes_into(item, out);
            }
        }
        if let Some(head) = node.get("head").and_then(Value::as_array) {
            for cell in head {
                collect_anchor_nodes_into(cell, out);
            }
        }
        if let Some(rows) = node.get("rows").and_then(Value::as_array) {
            for row in rows {
                if let Some(row) = row.as_array() {
                    for cell in row {
                        collect_anchor_nodes_into(cell, out);
                    }
                }
            }
        }
    }
}

fn find_search_entry<'a>(search_index: &'a Value, page_id: &str) -> &'a Value {
    // Root shape per `docs/architecture/json-renderer-contract.md`:
    // {"schema_version":1,"kind":"rhawiki_search_index","entries":[{"id",...}]}.
    search_index["entries"]
        .as_array()
        .expect("search index has an 'entries' array")
        .iter()
        .find(|entry| entry["id"] == page_id)
        .unwrap_or_else(|| panic!("search index has no entry for page {page_id}: {search_index}"))
}
