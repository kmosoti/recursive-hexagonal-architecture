//! Supplementary opaque-box end-to-end tests for CHG-011 (citations, `[Rn]`).
//!
//! Complements `crates/app-cli/tests/citations_render.rs` by covering additional
//! cross-feature interactions and edge scenarios:
//! 1. Transclusion citation rebasing (Feature 24): citations inside transcluded
//!    sections rebase their link targets to the origin page (`refs.html#ref-r1`).
//! 2. Duplicate reference entry demotion (Features 4, 5, 16): subsequent duplicate
//!    entries do not emit anchors, but their leading labels render as resolved links
//!    pointing to the first entry.
//! 3. Diverse contexts (Features 1, 2, 9, 11, 26): citations in table cells,
//!    glued text, letter suffixes (`[R1a]`), and list-embedded entries.
//! 4. Context exclusions (Feature 10): verifying that headings, code blocks,
//!    inline code, and markdown links suppress citation link generation in HTML output.
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
        "rha-citations-supp-{}-{}",
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

/// Tier 3: Cross-Feature Interaction — Transclusion Citation Rebasing (Feature 24).
/// Citations inside transcluded content must rebase their link target to the origin page
/// (`refs.html#ref-r1` in HTML, `refs.md#ref-r1` in JSON).
#[test]
fn transclusion_citation_rebasing_e2e() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    let json_output = temp.join("json");
    fs::create_dir(&source).unwrap();

    let refs_content = "\
# References Page

## Bibliography

[R1] Origin Author. (2020). Origin Reference.

## Discussion

This discussion cites [R1] on the origin page.
";

    let host_content = "\
# Host Page

Transcluding the discussion section from refs:

![[refs#discussion]]

End of host page.
";

    write_page(&source, "refs", refs_content);
    write_page(&source, "host", host_content);

    // Build HTML
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

    // Origin page has local anchor and intra-page link
    assert!(
        refs_html.contains("<a id=\"ref-r1\"></a>"),
        "refs.html: missing anchor ref-r1"
    );
    assert!(
        refs_html.contains("<a href=\"#ref-r1\">[R1]</a>"),
        "refs.html: intra-page citation link"
    );

    // Host page transcluding discussion has rebased citation link pointing back to origin page
    assert!(
        host_html.contains("<a href=\"refs.html#ref-r1\">[R1]</a>"),
        "host.html: citation in transcluded section must rebase to origin page: {host_html}"
    );
    // Host page must NOT duplicate the anchor
    assert!(
        !host_html.contains("id=\"ref-r1\""),
        "host.html: anchor ref-r1 must not be duplicated on host page"
    );

    // Build JSON
    let json_build = run_build(&source, &json_output, "json");
    assert!(
        json_build.status.success(),
        "JSON build failed: {}",
        String::from_utf8_lossy(&json_build.stderr)
    );

    let host_json: Value =
        serde_json::from_slice(&read_file(&json_output.join("host.json"))).expect("host.json");
    let host_anchors = collect_anchor_nodes(&host_json["body"]);
    assert!(
        host_anchors.is_empty(),
        "host.json: no anchor nodes should exist from transcluded content"
    );
}

/// Tier 3: Cross-Feature Interaction — Duplicate Reference Entry Citation Demotion (Features 4, 5, 16).
/// When duplicate entries share a label, the first wins as entry; the second becomes a citation
/// linking to the first, and only one anchor is emitted.
#[test]
fn duplicate_reference_entry_renders_demoted_citation_link() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    let json_output = temp.join("json");
    fs::create_dir(&source).unwrap();

    let dup_content = "\
# Duplicate References

[R1] Original reference author. (2020).

[R1] Duplicate impostor reference. (2022).
";

    write_page(&source, "dup", dup_content);

    let html = run_build(&source, &html_output, "html");
    assert!(
        html.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&html.stderr)
    );

    let dup_html =
        String::from_utf8(read_file(&html_output.join("dup.html"))).expect("dup.html UTF-8");

    // First entry has the anchor and plain leading label text
    assert!(
        dup_html.contains("<a id=\"ref-r1\"></a>"),
        "dup.html: must contain anchor for first entry"
    );

    // Second entry label demoted to citation link pointing to first entry
    assert!(
        dup_html.contains("<a href=\"#ref-r1\">[R1]</a> Duplicate impostor reference."),
        "dup.html: duplicate entry leading label must demote to citation link: {dup_html}"
    );

    // Verify anchor uniqueness: id="ref-r1" appears exactly once
    assert_eq!(
        dup_html.matches("id=\"ref-r1\"").count(),
        1,
        "dup.html: anchor id ref-r1 must appear exactly once"
    );

    // Build JSON and verify exactly one anchor node
    let json_build = run_build(&source, &json_output, "json");
    assert!(
        json_build.status.success(),
        "JSON build failed: {}",
        String::from_utf8_lossy(&json_build.stderr)
    );

    let dup_json: Value =
        serde_json::from_slice(&read_file(&json_output.join("dup.json"))).expect("dup.json");
    let anchors = collect_anchor_nodes(&dup_json["body"]);
    let anchor_ids: BTreeSet<String> = anchors
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        anchor_ids,
        BTreeSet::from(["ref-r1".to_owned()]),
        "dup.json: exactly one anchor node for ref-r1"
    );
    assert_eq!(anchors.len(), 1, "dup.json: total anchor count must be 1");

    // Verify check command exemption: duplicate reference entries are NOT check witnesses
    let check = run_check(&source);
    assert!(
        check.status.success(),
        "rhawiki check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let check_json: Value = serde_json::from_slice(&check.stdout).expect("check JSON");
    assert_eq!(
        check_json["witnesses"],
        Value::Array(vec![]),
        "duplicate reference entry must not be surfaced as a check witness"
    );
}

/// Tier 2: Boundary & Corner Cases — Diverse Citation Contexts & Inclusions (Features 1, 2, 9, 11, 26).
/// Citations in table cells, glued text (`word[R1a]glued`), and letter-suffix entries in lists.
#[test]
fn diverse_contexts_table_cells_and_glued_citations() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    fs::create_dir(&source).unwrap();

    let content = "\
# Diverse Contexts Page

## References

- [R1a] Letter suffix entry in list item.
- [R2] Numeric entry in list item.

## Table

| Header | Notes |
| --- | --- |
| Cell A | see [R1a] for details |
| Cell B | see [R2] for details |

## Glued Text

Here is word[R1a]glued on both sides and ([R2]) in parentheses.
";

    write_page(&source, "contexts", content);

    let html = run_build(&source, &html_output, "html");
    assert!(
        html.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&html.stderr)
    );

    let page_html = String::from_utf8(read_file(&html_output.join("contexts.html")))
        .expect("contexts.html UTF-8");

    // Anchors for list item entries
    assert!(
        page_html.contains("<a id=\"ref-r1a\"></a>"),
        "contexts.html: anchor for R1a with letter suffix"
    );
    assert!(
        page_html.contains("<a id=\"ref-r2\"></a>"),
        "contexts.html: anchor for R2"
    );

    // Citations inside table cells
    assert!(
        page_html.contains("<a href=\"#ref-r1a\">[R1a]</a>"),
        "contexts.html: citation in table cell for R1a"
    );
    assert!(
        page_html.contains("<a href=\"#ref-r2\">[R2]</a>"),
        "contexts.html: citation in table cell for R2"
    );

    // Glued text citations
    assert!(
        page_html.contains("word<a href=\"#ref-r1a\">[R1a]</a>glued"),
        "contexts.html: glued citation word[R1a]glued"
    );
    assert!(
        page_html.contains("(<a href=\"#ref-r2\">[R2]</a>)"),
        "contexts.html: citation inside parentheses ([R2])"
    );
}

/// Tier 2: Boundary & Corner Cases — Context Exclusions (Feature 10).
/// Verifies citations are never recognized in headings, code blocks, inline code, or link text.
#[test]
fn citation_context_exclusions_render_as_plain_text() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    fs::create_dir(&source).unwrap();

    let content = "\
# Exclusions Test

## Heading with [R1] must not cite

[R1] Reference Definition.

Inline code `[R1]` must not link.

```
[R1] inside fenced code
```

Link [R1](https://example.com) must not link to reference anchor.

Wikilink [[R1]] must not link to reference anchor.
";

    write_page(&source, "exclusions", content);

    let html = run_build(&source, &html_output, "html");
    assert!(
        html.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&html.stderr)
    );

    let page_html = String::from_utf8(read_file(&html_output.join("exclusions.html")))
        .expect("exclusions.html UTF-8");

    // The single valid anchor exists
    assert!(
        page_html.contains("<a id=\"ref-r1\"></a>"),
        "exclusions.html: missing entry anchor"
    );

    // No citation links should exist because all [R1] occurrences are in excluded contexts
    assert!(
        !page_html.contains("href=\"#ref-r1\""),
        "exclusions.html: no href=\"#ref-r1\" should be generated in excluded contexts: {page_html}"
    );

    // Code and heading text preserved literally
    assert!(
        page_html.contains("<code>[R1]</code>"),
        "exclusions.html: inline code preserved"
    );
    assert!(
        page_html.contains("[R1] inside fenced code"),
        "exclusions.html: fenced code preserved"
    );
}
