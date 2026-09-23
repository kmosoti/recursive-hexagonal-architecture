use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

#[path = "../../../xtask/tests/support/registered_package.rs"]
mod registered_package;

#[path = "../../../xtask/tests/support/registered_package_fs.rs"]
mod registered_package_fs;

fn read_file(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

struct OwnedTempDir(PathBuf);

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rha-transclusion-cli-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn write_sources(root: &Path, sources: &Value) {
    for source in sources.as_array().unwrap() {
        let path = root.join(format!("{}.md", source["path"].as_str().unwrap()));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, source["text"].as_str().unwrap()).unwrap();
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

fn push_node_text(
    node: &Value,
    headings: &mut Vec<String>,
    hrefs: &mut Vec<String>,
    images: &mut Vec<String>,
    text: &mut Vec<String>,
) {
    match node["type"].as_str().expect("node type") {
        "heading" => {
            headings.push(node["anchor"].as_str().unwrap().to_owned());
            push_nodes(&node["children"], headings, hrefs, images, text);
        }
        "paragraph" | "emphasis" | "strong" | "strikethrough" | "block_quote" => {
            push_nodes(&node["children"], headings, hrefs, images, text);
        }
        "text" | "code" | "html" => text.push(node["text"].as_str().unwrap().to_owned()),
        "code_block" => text.push(node["text"].as_str().unwrap().to_owned()),
        "link" => {
            hrefs.push(node["href"].as_str().unwrap().to_owned());
            push_nodes(&node["children"], headings, hrefs, images, text);
        }
        "wiki_link" => push_nodes(&node["children"], headings, hrefs, images, text),
        "image" => {
            images.push(node["src"].as_str().unwrap().to_owned());
            text.push(node["alt"].as_str().unwrap().to_owned());
        }
        "list" => {
            for item in node["items"].as_array().unwrap() {
                push_nodes(item, headings, hrefs, images, text);
            }
        }
        "table" => {
            for cell in node["head"].as_array().unwrap() {
                push_nodes(cell, headings, hrefs, images, text);
            }
            for row in node["rows"].as_array().unwrap() {
                for cell in row.as_array().unwrap() {
                    push_nodes(cell, headings, hrefs, images, text);
                }
            }
        }
        "task_marker" | "rule" => {}
        "soft_break" | "hard_break" => text.push(" ".to_owned()),
        other => panic!("unknown rendered node type {other}"),
    }
}

fn push_nodes(
    nodes: &Value,
    headings: &mut Vec<String>,
    hrefs: &mut Vec<String>,
    images: &mut Vec<String>,
    text: &mut Vec<String>,
) {
    for node in nodes.as_array().unwrap() {
        push_node_text(node, headings, hrefs, images, text);
    }
}

fn semantic_text(body: &Value) -> String {
    let mut headings = Vec::new();
    let mut hrefs = Vec::new();
    let mut images = Vec::new();
    let mut text = Vec::new();
    push_nodes(body, &mut headings, &mut hrefs, &mut images, &mut text);
    text.join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn observe_page(page: &Value, expected: &Value, id: &str) -> Vec<String> {
    let want_toc = expected["toc_anchors"].as_array().unwrap();
    let got_toc = page["toc"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| Value::String(entry["anchor"].as_str().unwrap().to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(&got_toc, want_toc, "{id}: TOC");

    let links = expected["wiki_targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|link| {
            json!({
                "status": "resolved",
                "page": link["page"],
                "anchor": link["anchor"],
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        page["links"],
        Value::Array(links),
        "{id}: complete links vector"
    );

    let mut headings = Vec::new();
    let mut hrefs = Vec::new();
    let mut images = Vec::new();
    let mut text = Vec::new();
    push_nodes(
        &page["body"],
        &mut headings,
        &mut hrefs,
        &mut images,
        &mut text,
    );
    assert_eq!(json!(headings), expected["heading_ids"], "{id}: headings");
    assert_eq!(json!(hrefs), expected["ordinary_hrefs"], "{id}: hrefs");
    assert_eq!(json!(images), expected["image_sources"], "{id}: images");
    let flattened = semantic_text(&page["body"]);
    for value in expected["text_includes"].as_array().unwrap() {
        assert!(
            flattened.contains(value.as_str().unwrap()),
            "{id}: missing {value}"
        );
    }
    for value in expected["text_excludes"].as_array().unwrap() {
        assert!(
            !flattened.contains(value.as_str().unwrap()),
            "{id}: found {value}"
        );
    }
    hrefs
}

#[test]
fn every_registered_site_is_checked_and_rendered_by_both_cli_formats() {
    let package = "sites";
    let cases = registered_package::load(package);
    let mut completed = BTreeSet::new();
    for case in &cases {
        assert_eq!(case["kind"], "site");
        let id = case["id"].as_str().unwrap();
        assert!(completed.insert(id.to_owned()));
        let temp = temp_root();
        let _guard = OwnedTempDir(temp.clone());
        let source = temp.join("source");
        let html_output = temp.join("html");
        let json_output = temp.join("json");
        fs::create_dir(&source).unwrap();
        write_sources(&source, &case["sources"]);

        let check = run_check(&source);
        assert_eq!(
            check.status.code(),
            Some(case["expected_exit"].as_i64().unwrap() as i32),
            "{id}: check exit"
        );
        let actual_check: Value = serde_json::from_slice(&check.stdout).expect("check JSON");
        assert_eq!(actual_check, case["expected_check"], "{id}: check JSON");

        let html = run_build(&source, &html_output, "html");
        assert!(
            html.status.success(),
            "{id}: HTML build failed: {}",
            String::from_utf8_lossy(&html.stderr)
        );
        let json_build = run_build(&source, &json_output, "json");
        assert!(
            json_build.status.success(),
            "{id}: JSON build failed: {}",
            String::from_utf8_lossy(&json_build.stderr)
        );

        let mut html_pages = BTreeMap::new();
        let mut observed_hrefs = BTreeMap::<String, Vec<String>>::new();
        for (page_id, expected) in case["expected_pages"].as_object().unwrap() {
            let json_page: Value =
                serde_json::from_slice(&read_file(&json_output.join(format!("{page_id}.json"))))
                    .expect("page JSON");
            let hrefs = observe_page(&json_page, expected, page_id);
            let html_text =
                String::from_utf8(read_file(&html_output.join(format!("{page_id}.html"))))
                    .expect("HTML UTF-8");
            html_pages.insert(page_id.clone(), html_text);
            observed_hrefs.insert(page_id.clone(), hrefs);
        }

        for (page_id, html_text) in &html_pages {
            let expected = &case["expected_pages"][page_id];
            for marker in expected["text_includes"].as_array().unwrap() {
                let marker = marker.as_str().unwrap();
                if marker.starts_with("[transclusion unavailable:")
                    || marker.starts_with("[transclusion cycle:")
                {
                    let escaped = marker
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                        .replace('"', "&quot;")
                        .replace('\'', "&#39;");
                    assert!(
                        html_text.contains(&escaped),
                        "{id}/{page_id}: missing HTML marker {marker}"
                    );
                    let json_page: Value = serde_json::from_slice(&read_file(
                        &json_output.join(format!("{page_id}.json")),
                    ))
                    .unwrap();
                    assert!(
                        semantic_text(&json_page["body"]).contains(marker),
                        "{id}/{page_id}: missing JSON marker {marker}"
                    );
                }
            }
        }

        if let Some(mapping) = case.get("expected_html_contains") {
            let mut covered = BTreeSet::new();
            for (href, marker) in mapping.as_object().unwrap() {
                let mut found = false;
                for (page_id, hrefs) in &observed_hrefs {
                    if hrefs.iter().any(|actual| actual == href) {
                        found = true;
                        let escaped = marker
                            .as_str()
                            .unwrap()
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                            .replace('"', "&quot;")
                            .replace('\'', "&#39;");
                        let attribute = format!("href=\"{escaped}\"");
                        assert!(
                            html_pages[page_id].contains(&attribute),
                            "{id}/{page_id}: missing HTML rewrite for {href}"
                        );
                    }
                }
                assert!(found, "{id}: expected HTML key {href} was not observed");
                assert!(covered.insert(href.clone()));
            }
            assert_eq!(covered.len(), mapping.as_object().unwrap().len());
        }
    }
    assert_eq!(completed.len(), 12);
}

fn copy_tree(source: &Path, target: &Path) {
    let metadata = fs::symlink_metadata(source).expect("fixture metadata");
    assert!(!metadata.file_type().is_symlink());
    assert!(metadata.is_dir());
    fs::create_dir_all(target).expect("fixture target directory");
    for entry in fs::read_dir(source).expect("fixture entries") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).expect("fixture child metadata");
        assert!(!metadata.file_type().is_symlink());
        if metadata.is_dir() {
            copy_tree(&source_path, &target_path);
        } else {
            assert!(metadata.is_file());
            fs::copy(&source_path, &target_path).expect("fixture payload copy");
        }
    }
}

#[test]
fn registered_embedded_generation_is_current() {
    let output = Command::new("python3")
        .args([
            "-B",
            "xtask/tests/support/generate_registered_embedded.py",
            "--check",
        ])
        .current_dir(repository_root())
        .output()
        .expect("spawn registered embedded generator");
    assert!(
        output.status.success(),
        "registered embedded generator check failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fixture_valid_control_is_accepted_before_corruption_is_rejected() {
    let source = registered_package_fs::root("sections");
    registered_package_fs::load_at(&source, "sections").expect("valid registered control");
    let temporary = temp_root();
    let _guard = OwnedTempDir(temporary.clone());
    let copied = temporary.join("sections");
    copy_tree(&source, &copied);

    registered_package_fs::load_at(&copied, "sections").expect("valid copied control");

    let cases = copied.join("CASES.json");
    let mut bytes = fs::read(&cases).expect("copied cases");
    bytes.extend_from_slice(b"\n");
    fs::write(&cases, bytes).expect("corrupt copied cases");
    let error = registered_package_fs::load_at(&copied, "sections").expect_err("corruption");
    assert_eq!(error, "CASES.json digest is not the registered digest");
}

#[cfg(unix)]
#[test]
fn fixture_symlink_is_rejected_before_any_payload_read() {
    use std::os::unix::fs::symlink;

    let source = registered_package_fs::root("sections");
    registered_package_fs::load_at(&source, "sections").expect("valid registered control");
    let temporary = temp_root();
    let _guard = OwnedTempDir(temporary.clone());
    let copied = temporary.join("sections");
    copy_tree(&source, &copied);

    registered_package_fs::load_at(&copied, "sections").expect("valid copied control");

    let payload = copied.join("source-snapshots").join("bdr.md");
    let outside = temporary.join("bdr.md");
    fs::copy(&payload, &outside).expect("byte-identical outside payload");
    fs::remove_file(&payload).expect("replace payload");
    symlink(&outside, &payload).expect("symlink fixture");
    let error = registered_package_fs::load_at(&copied, "sections").expect_err("symlink");
    assert!(
        error.starts_with("symlink found before payload read: "),
        "unexpected error: {error}"
    );

    let cases = copied.join("CASES.json");
    let mut bytes = fs::read(&cases).expect("copied cases");
    bytes[0] = b' ';
    fs::write(&cases, bytes).expect("poison copied cases");
    let error =
        registered_package_fs::load_at(&copied, "sections").expect_err("symlink precedence");
    assert!(
        error.starts_with("symlink found before payload read: "),
        "unexpected error: {error}"
    );
}
