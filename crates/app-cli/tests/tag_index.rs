//! End-to-end grader for CHG-013 stage B (the tag index page), written
//! before any implementation exists, from
//! `docs/architecture/tag-index-contract.md` alone.
//!
//! Follows the conventions of `crates/app-cli/tests/citations_render.rs`
//! and `crates/app-cli/tests/callouts_render.rs`: a standalone docs tree in
//! a temp dir (not the registered `xtask/tests/corpus/*` embedded-payload
//! convention used by `transclusion.rs`), built and checked through the
//! `rhawiki` binary via `CARGO_BIN_EXE_rhawiki`. Unlike those two files,
//! this grader drives a whole *site* corpus (`xtask/tests/corpus/tag-index/
//! CASES.json`, one independent Python reference package per case), so it
//! embeds that corpus with `include_str!` and verifies its `SHA256SUMS`
//! digest for `CASES.json` before trusting any of it (the same principle
//! `xtask/tests/support/registered_package.rs` applies to the corpora
//! already wired into the workspace's build script, read here for that
//! convention only).
//!
//! `app-cli` is not a core crate, so this file uses `std::fs` freely
//! (AGENTS.md: "Core crates perform no I/O ... effects cross ports";
//! adapters, including the CLI, are where I/O happens).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

/// The registered case corpus and its manifest, embedded so this grader
/// carries its own oracle rather than reading it from disk at test time.
/// Path is relative to this file: `crates/app-cli/tests/tag_index.rs` ->
/// `xtask/tests/corpus/tag-index/`.
const CASES_JSON: &str = include_str!("../../../xtask/tests/corpus/tag-index/CASES.json");
const SHA256SUMS: &str = include_str!("../../../xtask/tests/corpus/tag-index/SHA256SUMS");

/// Verifies that the embedded `CASES.json` bytes match the digest
/// registered for it in the embedded `SHA256SUMS`, so a silent edit to
/// either file (without regenerating the other) fails loudly instead of
/// grading against drifted expectations.
fn verify_embedded_payload() {
    let mut found = None;
    for line in SHA256SUMS.lines() {
        let (digest, relative) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("malformed SHA256SUMS line: {line:?}"));
        if relative == "CASES.json" {
            found = Some(digest.to_owned());
        }
    }
    let expected = found.expect("SHA256SUMS has no entry for CASES.json");
    let actual = library::Digest::of(CASES_JSON.as_bytes()).to_string();
    assert_eq!(
        actual, expected,
        "embedded CASES.json does not match its registered SHA256SUMS digest: \
         the corpus and its manifest have drifted apart"
    );
}

fn temp_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rha-tag-index-cli-{}-{}",
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

fn write_page(root: &Path, page_id: &str, text: &str) {
    let path = root.join(format!("{page_id}.md"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

/// Contract section 1: `index`'s value is a scalar at column 1, so a
/// front-matter `index:` line is always exactly one whole line starting
/// with the literal `index:`. Removing every such line from a page's
/// source is "the index line removed from its front matter": the
/// resulting page has no `index` key at all (so it is never a tag index),
/// regardless of how many `index:` lines the original page had -- this
/// grader's registered corpus never relies on more than that.
fn without_index_lines(markdown: &str) -> String {
    markdown
        .lines()
        .filter(|line| !line.starts_with("index:"))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn write_tree(root: &Path, pages: &serde_json::Map<String, Value>, strip_index: bool) {
    for (page_id, markdown) in pages {
        let text = markdown.as_str().expect("page markdown is a string");
        let text = if strip_index {
            without_index_lines(text)
        } else {
            text.to_owned()
        };
        write_page(root, page_id, &text);
    }
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

fn read_page_json(output_root: &Path, page_id: &str) -> Value {
    let path = output_root.join(format!("{page_id}.json"));
    serde_json::from_slice(&read_file(&path))
        .unwrap_or_else(|error| panic!("{}: not JSON: {error}", path.display()))
}

/// Full-object comparison with `built_at` normalized away: the JSON
/// contract's only clock-derived member (`docs/architecture/
/// json-renderer-contract.md`, "Encoding and determinism").
fn without_built_at(mut page: Value) -> Value {
    if let Some(object) = page.as_object_mut() {
        object.insert("built_at".to_owned(), Value::Null);
    }
    page
}

struct ExpectedLink {
    page_id: String,
    link_text: String,
}

struct ExpectedSection {
    heading_text: String,
    slug: String,
    links: Vec<ExpectedLink>,
}

struct ExpectedIndexPage {
    sections: Vec<ExpectedSection>,
}

fn parse_expected_index_page(value: &Value) -> ExpectedIndexPage {
    let sections = value["sections"]
        .as_array()
        .expect("expected_index_pages[].sections is an array")
        .iter()
        .map(|section| ExpectedSection {
            heading_text: section["heading_text"].as_str().unwrap().to_owned(),
            slug: section["slug"].as_str().unwrap().to_owned(),
            links: section["links"]
                .as_array()
                .expect("section.links is an array")
                .iter()
                .map(|link| ExpectedLink {
                    page_id: link["page_id"].as_str().unwrap().to_owned(),
                    link_text: link["link_text"].as_str().unwrap().to_owned(),
                })
                .collect(),
        })
        .collect();
    ExpectedIndexPage { sections }
}

fn text_children(text: &str) -> Value {
    serde_json::json!([{"type": "text", "text": text}])
}

/// Checks the generated section on one tag-index page's JSON against its
/// `ExpectedIndexPage`, and pins that nothing else on the page changed
/// (contract section 4: "A tag index page renders exactly as a page
/// without `index: tags` would, apart from the generated section").
///
/// `page_a` is the page as built by the site under test; `page_b` is the
/// same page as built with the `index` line removed from its front
/// matter, so `page_b`'s body/TOC/links are exactly "before the appended
/// part" for `page_a` (contract section 3: "assembly appends ... after
/// its own body").
fn check_generated_section(
    case_id: &str,
    page_id: &str,
    expected: &ExpectedIndexPage,
    page_a: &Value,
    page_b: &Value,
) {
    let body_a = page_a["body"].as_array().expect("page.body is an array");
    let body_b = page_b["body"].as_array().expect("page.body is an array");
    let toc_a = page_a["toc"].as_array().expect("page.toc is an array");
    let toc_b = page_b["toc"].as_array().expect("page.toc is an array");
    let links_a = page_a["links"].as_array().expect("page.links is an array");
    let links_b = page_b["links"].as_array().expect("page.links is an array");

    let generated_node_count = expected.sections.len() * 2;
    assert_eq!(
        body_a.len(),
        body_b.len() + generated_node_count,
        "{case_id}/{page_id}: body must gain exactly one heading and one list per generated \
         tag section (found body_a.len()={}, body_b.len()={}, expected sections={})",
        body_a.len(),
        body_b.len(),
        expected.sections.len()
    );
    assert_eq!(
        &body_a[..body_b.len()],
        body_b.as_slice(),
        "{case_id}/{page_id}: the body before the appended part must equal the body of the \
         same page built with the index line removed from its front matter"
    );

    assert_eq!(
        toc_a.len(),
        toc_b.len() + expected.sections.len(),
        "{case_id}/{page_id}: TOC must gain exactly one entry per generated tag section"
    );
    assert_eq!(
        &toc_a[..toc_b.len()],
        toc_b.as_slice(),
        "{case_id}/{page_id}: the TOC before the generated tail must equal the TOC of the \
         same page built with the index line removed"
    );

    let pre_existing_link_count = links_b.len();
    assert_eq!(
        &links_a[..pre_existing_link_count],
        links_b.as_slice(),
        "{case_id}/{page_id}: pre-existing links must be unchanged, in the same order, once \
         the generated section's links are appended"
    );

    let mut next_expected_link_index = pre_existing_link_count;

    for (i, section) in expected.sections.iter().enumerate() {
        let heading = &body_a[body_b.len() + 2 * i];
        assert_eq!(
            heading["type"], "heading",
            "{case_id}/{page_id}: generated node {i} must be a heading: {heading}"
        );
        assert_eq!(
            heading["level"], 2,
            "{case_id}/{page_id}: generated tag heading must be level 2: {heading}"
        );
        assert_eq!(
            heading["anchor"], section.slug,
            "{case_id}/{page_id}: generated tag heading anchor for {:?}",
            section.heading_text
        );
        assert_eq!(
            heading["children"],
            text_children(&section.heading_text),
            "{case_id}/{page_id}: generated tag heading text for {:?}",
            section.heading_text
        );

        let toc_entry = &toc_a[toc_b.len() + i];
        assert_eq!(
            *toc_entry,
            serde_json::json!({"level": 2, "text": section.heading_text, "anchor": section.slug}),
            "{case_id}/{page_id}: TOC tail entry for {:?}",
            section.heading_text
        );

        let list = &body_a[body_b.len() + 2 * i + 1];
        assert_eq!(
            list["type"],
            "list",
            "{case_id}/{page_id}: generated node {} must be a list: {list}",
            2 * i + 1
        );
        assert_eq!(
            list["start"],
            Value::Null,
            "{case_id}/{page_id}: generated list must have start=null"
        );
        let items = list["items"].as_array().expect("list.items is an array");
        assert_eq!(
            items.len(),
            section.links.len(),
            "{case_id}/{page_id}: tag {:?} must list exactly its pages",
            section.heading_text
        );

        for (j, (item, expected_link)) in items.iter().zip(section.links.iter()).enumerate() {
            let item_nodes = item.as_array().expect("list item is an array of nodes");
            assert_eq!(
                item_nodes.len(),
                1,
                "{case_id}/{page_id}: tag {:?} item {j} must be a single node",
                section.heading_text
            );
            let wiki_link = &item_nodes[0];
            assert_eq!(
                wiki_link["type"], "wiki_link",
                "{case_id}/{page_id}: tag {:?} item {j} must be a wiki_link: {wiki_link}",
                section.heading_text
            );
            assert_eq!(
                wiki_link["children"],
                text_children(&expected_link.link_text),
                "{case_id}/{page_id}: tag {:?} item {j} link text",
                section.heading_text
            );
            let link_index = wiki_link["link_index"]
                .as_u64()
                .unwrap_or_else(|| panic!("{case_id}/{page_id}: wiki_link has no link_index"));
            assert_eq!(
                link_index, next_expected_link_index as u64,
                "{case_id}/{page_id}: tag {:?} item {j} must use the next appended link index \
                 (appended, not interleaved with or reordering pre-existing links)",
                section.heading_text
            );
            next_expected_link_index += 1;

            let resolved = links_a.get(link_index as usize).unwrap_or_else(|| {
                panic!("{case_id}/{page_id}: link_index {link_index} out of range of links[]")
            });
            assert_eq!(
                *resolved,
                serde_json::json!({
                    "status": "resolved",
                    "page": expected_link.page_id,
                    "anchor": Value::Null,
                }),
                "{case_id}/{page_id}: tag {:?} item {j} must resolve to page {:?} with no anchor",
                section.heading_text,
                expected_link.page_id
            );
        }
    }

    assert_eq!(
        links_a.len(),
        next_expected_link_index,
        "{case_id}/{page_id}: links[] must contain exactly the pre-existing links plus one \
         appended entry per generated wikilink, no more"
    );
}

#[test]
fn tag_index_generated_sections_match_the_registered_corpus() {
    verify_embedded_payload();

    let cases: Vec<Value> = serde_json::from_str(CASES_JSON).expect("CASES.json is valid JSON");
    assert!(
        cases.len() >= 25,
        "the registered corpus must have at least 25 cases"
    );

    let mut ids = std::collections::BTreeSet::new();
    let mut total_index_pages = 0usize;

    for case in &cases {
        let case_id = case["id"].as_str().expect("case.id is a string");
        assert!(
            ids.insert(case_id.to_owned()),
            "duplicate case id: {case_id}"
        );

        let pages = case["pages"]
            .as_object()
            .unwrap_or_else(|| panic!("{case_id}: pages must be an object"));
        let expected_index_pages: BTreeMap<String, ExpectedIndexPage> =
            case["expected_index_pages"]
                .as_object()
                .unwrap_or_else(|| panic!("{case_id}: expected_index_pages must be an object"))
                .iter()
                .map(|(page_id, value)| (page_id.clone(), parse_expected_index_page(value)))
                .collect();
        total_index_pages += expected_index_pages.len();

        let temp = temp_root();
        let _guard = OwnedTempDir(temp.clone());
        let source_a = temp.join("source-a");
        let source_b = temp.join("source-b");
        let out_a = temp.join("out-a");
        let out_b = temp.join("out-b");
        let out_html = temp.join("out-html");
        fs::create_dir(&source_a).unwrap();
        fs::create_dir(&source_b).unwrap();

        write_tree(&source_a, pages, false);
        write_tree(&source_b, pages, true);

        let build_a = run_build(&source_a, &out_a, "json");
        assert!(
            build_a.status.success(),
            "{case_id}: JSON build (with index) failed: {}",
            String::from_utf8_lossy(&build_a.stderr)
        );
        let build_b = run_build(&source_b, &out_b, "json");
        assert!(
            build_b.status.success(),
            "{case_id}: JSON build (index line removed) failed: {}",
            String::from_utf8_lossy(&build_b.stderr)
        );

        for page_id in pages.keys() {
            let page_a = read_page_json(&out_a, page_id);
            let page_b = read_page_json(&out_b, page_id);

            if let Some(expected) = expected_index_pages.get(page_id) {
                check_generated_section(case_id, page_id, expected, &page_a, &page_b);
            } else {
                // Contract section 4: "every page that is not a tag index
                // produces exactly the page model it produced before this
                // stage." Removing an unrelated (or absent) `index:` line
                // must therefore change nothing observable about this page.
                assert_eq!(
                    without_built_at(page_a.clone()),
                    without_built_at(page_b.clone()),
                    "{case_id}/{page_id}: a non-tag-index page's JSON must be identical \
                     whether or not any `index:` line is present"
                );
            }
        }

        // One HTML check per case: every generated heading has its <h2
        // id="SLUG"> and every generated link has an <a href=...> to its
        // target (contract section 3).
        let build_html = run_build(&source_a, &out_html, "html");
        assert!(
            build_html.status.success(),
            "{case_id}: HTML build failed: {}",
            String::from_utf8_lossy(&build_html.stderr)
        );
        for (page_id, expected) in &expected_index_pages {
            let html_path = out_html.join(format!("{page_id}.html"));
            let html = String::from_utf8(read_file(&html_path))
                .unwrap_or_else(|error| panic!("{case_id}/{page_id}.html UTF-8: {error}"));
            for section in &expected.sections {
                let heading_marker = format!("<h2 id=\"{}\">", section.slug);
                assert!(
                    html.contains(&heading_marker),
                    "{case_id}/{page_id}.html: missing generated heading marker {heading_marker:?}: {html}"
                );
                for link in &section.links {
                    // A generated wikilink to the index page itself renders
                    // with an empty href, the same self-link convention the
                    // HTML renderer already uses for any wikilink whose
                    // target is the current page
                    // (`crates/adapter-html/src/lib.rs`, read for this
                    // already-implemented, unrelated rendering detail
                    // only); every other target must be visibly linked by
                    // its rendered `.html` filename.
                    if link.page_id == *page_id {
                        assert!(
                            html.contains("<a class=\"wikilink\" href=\"\">"),
                            "{case_id}/{page_id}.html: missing a self-link <a> for {:?}: {html}",
                            link.page_id
                        );
                        continue;
                    }
                    let basename = link.page_id.rsplit('/').next().unwrap_or(&link.page_id);
                    let href_marker = format!("{basename}.html");
                    assert!(
                        html.contains("<a href=") || html.contains("<a class=\"wikilink\" href="),
                        "{case_id}/{page_id}.html: no anchor tag at all: {html}"
                    );
                    assert!(
                        html.contains(&href_marker),
                        "{case_id}/{page_id}.html: missing a link to {href_marker:?}: {html}"
                    );
                }
            }
        }

        // `rhawiki check --format json` must report zero witnesses: every
        // registered case is a clean site (no broken links, ambiguous
        // links, missing anchors, or duplicate slugs), and the tag-index
        // feature adds no new check witness (decision
        // `tag-index-check-surface`).
        let check = run_check(&source_a);
        assert!(
            check.status.success(),
            "{case_id}: check failed unexpectedly: {}",
            String::from_utf8_lossy(&check.stderr)
        );
        let check_json: Value = serde_json::from_slice(&check.stdout)
            .unwrap_or_else(|error| panic!("{case_id}: check JSON: {error}"));
        assert_eq!(
            check_json["witnesses"],
            Value::Array(vec![]),
            "{case_id}: a clean site must report zero check witnesses: {check_json}"
        );
        assert_eq!(
            check_json["counts"],
            serde_json::json!({}),
            "{case_id}: a clean site must report empty check counts: {check_json}"
        );
    }

    assert!(
        total_index_pages > 0,
        "at least one case must exercise a tag-index page with a generated section"
    );
}
