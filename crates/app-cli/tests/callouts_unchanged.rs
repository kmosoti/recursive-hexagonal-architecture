//! "Nothing else changes" for CHG-012 (callouts): contract section 3, checked against output
//! captured with the renderer before the feature (at 558987a, the contract commit).
//!
//! The fixture docs tree `tests/data/callouts_unchanged/docs` holds a page without callouts
//! (headings, lists, tables, code, plain quotes, links, wikilinks, citations, §-references, `![`),
//! a page of callouts and non-callouts, and a page that transcludes a callout. After the feature:
//! - every JSON output file (pages and the search index) is byte-identical;
//! - every HTML page is byte-identical once `<p class="callout-title">…</p>` lines are removed, so
//!   a page without callouts is byte-identical and a callout page differs only by title lines;
//! - the old stylesheet's lines all appear, in order, in the new one (rules are only added).
//!
//! The build timestamp is the one varying field; it is blanked before comparing. Set
//! `RHAWIKI_CAPTURE_CALLOUT_GOLDEN=1` to rewrite the golden file (done once, before the feature).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const GOLDEN: &str = include_str!("data/callouts_unchanged/golden.json");

struct OwnedTempDir(PathBuf);

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp(tag: &str) -> OwnedTempDir {
    let path = std::env::temp_dir().join(format!(
        "rha-callouts-unchanged-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    OwnedTempDir(path)
}

fn docs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/callouts_unchanged/docs")
}

fn build(out: &Path, format: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_rhawiki"))
        .args(["build", "--root"])
        .arg(docs())
        .args(["--out"])
        .arg(out)
        .args(["--format", format])
        .output()
        .expect("rhawiki build must start");
    assert!(
        output.status.success(),
        "{format} build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn files(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(root, &path, out);
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, fs::read_to_string(&path).unwrap());
        }
    }
}

/// Blank the build timestamp: the HTML footer line and the JSON `built_at` value.
fn normalise(text: &str) -> String {
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        if line.starts_with("<footer>Built at ") {
            out.push_str("<footer>Built at *</footer>\n");
        } else {
            out.push_str(line);
        }
    }
    if let Some(start) = out.find("\"built_at\":") {
        let value_start = start + "\"built_at\":".len();
        let rest = &out[value_start..];
        let value_len = rest.find([',', '}']).unwrap_or(rest.len());
        out.replace_range(value_start..value_start + value_len, "\"*\"");
    }
    out
}

fn without_callout_titles(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| {
            let line = line.trim_end();
            !(line.starts_with("<p class=\"callout-title\">") && line.ends_with("</p>"))
        })
        .collect()
}

fn observed() -> BTreeMap<String, String> {
    let html = temp("html");
    let json = temp("json");
    build(&html.0, "html");
    build(&json.0, "json");
    let mut out = BTreeMap::new();
    let mut html_files = BTreeMap::new();
    files(&html.0, &html.0, &mut html_files);
    for (path, text) in html_files {
        out.insert(format!("html/{path}"), normalise(&text));
    }
    let mut json_files = BTreeMap::new();
    files(&json.0, &json.0, &mut json_files);
    for (path, text) in json_files {
        out.insert(format!("json/{path}"), normalise(&text));
    }
    out
}

#[test]
fn output_is_unchanged_except_callout_titles_and_added_styles() {
    let observed = observed();
    if std::env::var_os("RHAWIKI_CAPTURE_CALLOUT_GOLDEN").is_some() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/callouts_unchanged/golden.json");
        fs::write(
            path,
            serde_json::to_string_pretty(&observed).unwrap() + "\n",
        )
        .unwrap();
        return;
    }
    let golden: BTreeMap<String, String> = serde_json::from_str(GOLDEN).unwrap();
    assert_eq!(golden.len(), 8, "golden file count");
    assert_eq!(
        golden.keys().collect::<Vec<_>>(),
        observed.keys().collect::<Vec<_>>(),
        "the same output files"
    );
    for (path, old) in &golden {
        let new = &observed[path];
        if path.ends_with("style.css") {
            let mut new_lines = new.lines();
            for old_line in old.lines() {
                assert!(
                    new_lines.any(|line| line == old_line),
                    "{path}: the old rule line {old_line:?} is missing or out of order"
                );
            }
        } else if path.starts_with("html/") {
            assert_eq!(
                &without_callout_titles(new),
                old,
                "{path}: HTML beyond callout titles"
            );
        } else {
            assert_eq!(new, old, "{path}: JSON output");
        }
    }
    let plain = &observed["html/plain.html"];
    assert!(
        !plain.contains("class=\"callout"),
        "the page without callouts has no callout markup"
    );
}
