//! End-to-end grader for CHG-012 (GFM callout titles and per-kind styling),
//! written before any implementation exists, from
//! `docs/architecture/callout-contract.md` alone.
//!
//! Follows the conventions of `crates/app-cli/tests/citations_render.rs`:
//! a standalone docs tree in a temp dir (not the registered
//! `xtask/tests/corpus/*` convention), built and checked through the
//! `rhawiki` binary via `CARGO_BIN_EXE_rhawiki`.
//!
//! `app-cli` is not a core crate, so this file uses `std::fs` freely
//! (AGENTS.md: "Core crates perform no I/O ... effects cross ports";
//! adapters, including the CLI, are where I/O happens).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

fn temp_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rha-callouts-cli-{}-{}",
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

/// The five kinds, their contract-mandated lowercase HTML class and title
/// label, in one place so every test iterates the same source of truth.
const KINDS: [(&str, &str); 5] = [
    ("note", "Note"),
    ("tip", "Tip"),
    ("important", "Important"),
    ("warning", "Warning"),
    ("caution", "Caution"),
];

/// Three marker spellings per kind (contract section 1: "in any letter
/// case"): upper, lower and a mixed case that is neither the upper form
/// nor the contract's title-case label, so a renderer that merely
/// echoes the marker's own case could not accidentally match the
/// expected title.
fn marker_cases(kind_upper: &str) -> [String; 3] {
    let lower = kind_upper.to_lowercase();
    let mixed: String = kind_upper
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 0 {
                c.to_ascii_lowercase()
            } else {
                c.to_ascii_uppercase()
            }
        })
        .collect();
    [kind_upper.to_owned(), lower, mixed]
}

/// Builds the shared fixture tree:
/// - `cases.md`: all five kinds, three marker casings each (15 callouts).
/// - `structure.md`: an empty-body callout, nested callouts, a callout
///   inside a list item, and a callout whose body has a list, a code
///   block and a link.
/// - `not_callouts.md`: the contract section 1 negative examples plus a
///   plain block quote.
/// - `json_callout.md`: one callout whose page text never spells a kind
///   label outside its own marker, for the JSON/search-text assertions.
fn build_tree(source: &Path) {
    write_page(source, "cases", &cases_markdown());
    write_page(source, "structure", STRUCTURE_MARKDOWN);
    write_page(source, "not_callouts", NOT_CALLOUTS_MARKDOWN);
    write_page(source, "json_callout", JSON_CALLOUT_MARKDOWN);
}

fn cases_markdown() -> String {
    let mut out = String::new();
    for (kind_lower, _label) in KINDS {
        let kind_upper = kind_lower.to_uppercase();
        for (case_name, marker) in ["upper", "lower", "mixed"]
            .iter()
            .zip(marker_cases(&kind_upper))
        {
            out.push_str(&format!(
                "> [!{marker}]\n> {kind_lower} {case_name} case body text.\n\n"
            ));
        }
    }
    out
}

/// The exact fragment the contract requires for one rendered callout with
/// a single-paragraph body: the opening tag, the title paragraph as the
/// first child, then the body, then the closing tag.
fn callout_fragment(kind_lower: &str, label: &str, body_paragraph: &str) -> String {
    format!(
        "<blockquote class=\"callout {kind_lower}\">\n<p class=\"callout-title\">{label}</p>\n<p>{body_paragraph}</p>\n</blockquote>"
    )
}

const STRUCTURE_MARKDOWN: &str = "\
> [!TIP]

> [!WARNING]
> Outer warning body sentence.
>
> > [!CAUTION]
> > Inner caution body sentence.

- List item one text.
- > [!IMPORTANT]
  > List item callout body sentence.
- List item three text.

> [!NOTE]
> Rich body intro sentence.
>
> - Rich list item alpha.
> - Rich list item beta.
>
> ```
> rich code block line
> ```
>
> [Rich example link](https://example.org/rich)
";

const NOT_CALLOUTS_MARKDOWN: &str = "\
> [!NOTE] Title on the marker line
> Body after a titled marker line.

> [!info]
> Body of an unrecognised-kind quote.

> [!note]-
> Body of a fold-marked quote.

> Body of a plain block quote with no marker.
";

const JSON_CALLOUT_MARKDOWN: &str = "\
# Json Callout Fixture

> [!NOTE]
> alpha bravo charlie delta echo.
";

/// Words the JSON/search-text assertions must never find outside a
/// marker: the contract's five title-case labels, checked
/// case-insensitively so a differently-cased accidental label collision
/// (e.g. from a generated title) is also caught.
const LABEL_WORDS: [&str; 5] = ["note", "tip", "important", "warning", "caution"];

#[test]
fn callouts_render_title_for_every_kind_and_marker_case() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    fs::create_dir(&source).unwrap();
    write_page(&source, "cases", &cases_markdown());

    let build = run_build(&source, &html_output, "html");
    assert!(
        build.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let html = String::from_utf8(read_file(&html_output.join("cases.html"))).expect("UTF-8 HTML");

    for (kind_lower, label) in KINDS {
        for case_name in ["upper", "lower", "mixed"] {
            let body = format!("{kind_lower} {case_name} case body text.");
            let fragment = callout_fragment(kind_lower, label, &body);
            assert!(
                html.contains(&fragment),
                "kind {kind_lower}, marker case {case_name}: expected the fragment \
                 {fragment:?} (title immediately after the opening blockquote tag, \
                 lowercase kind class, capitalised label) but it was not found in \
                 cases.html:\n{html}"
            );
        }
    }
}

#[test]
fn callouts_render_empty_body_nesting_list_items_and_rich_bodies() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    fs::create_dir(&source).unwrap();
    write_page(&source, "structure", STRUCTURE_MARKDOWN);

    let build = run_build(&source, &html_output, "html");
    assert!(
        build.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let html =
        String::from_utf8(read_file(&html_output.join("structure.html"))).expect("UTF-8 HTML");

    // Empty body (`> [!TIP]` alone): the title is still present, and is
    // immediately followed by the closing tag (contract section 2: "The
    // title paragraph is the first child and appears even when the body
    // is empty.").
    assert!(
        html.contains(
            "<blockquote class=\"callout tip\">\n<p class=\"callout-title\">Tip</p>\n</blockquote>"
        ),
        "empty-body callout (`> [!TIP]` alone): expected a title with no body children \
         between it and the closing tag, but it was not found in structure.html:\n{html}"
    );

    // Nested callouts: each gets its own title (contract section 2).
    assert!(
        html.contains(
            "<blockquote class=\"callout warning\">\n<p class=\"callout-title\">Warning</p>\n<p>Outer warning body sentence.</p>\n"
        ),
        "nested callouts, outer warning: expected the outer callout's own title \
         immediately after its opening tag, but it was not found in structure.html:\n{html}"
    );
    assert!(
        html.contains(
            "<blockquote class=\"callout caution\">\n<p class=\"callout-title\">Caution</p>\n<p>Inner caution body sentence.</p>\n</blockquote>"
        ),
        "nested callouts, inner caution: expected the inner callout's own title \
         immediately after its own opening tag, but it was not found in structure.html:\n{html}"
    );
    let outer_open = html
        .find("<blockquote class=\"callout warning\">")
        .expect("nested callouts: outer warning blockquote must be present");
    let inner_open = html
        .find("<blockquote class=\"callout caution\">")
        .expect("nested callouts: inner caution blockquote must be present");
    let outer_close = html[outer_open..]
        .find("</blockquote>\n</blockquote>")
        .map(|i| i + outer_open)
        .expect("nested callouts: outer blockquote must close after the inner one");
    assert!(
        inner_open > outer_open && inner_open < outer_close,
        "nested callouts: the inner caution callout must be nested inside the outer \
         warning callout, not a sibling: outer_open={outer_open}, inner_open={inner_open}, \
         outer_close={outer_close}"
    );

    // A callout in a list item: titled (contract section 1: "Callouts can
    // ... sit in list items").
    assert!(
        html.contains(
            "<li><blockquote class=\"callout important\">\n<p class=\"callout-title\">Important</p>\n<p>List item callout body sentence.</p>\n</blockquote>\n</li>"
        ),
        "callout in a list item: expected a titled callout as the list item's content, \
         but it was not found in structure.html:\n{html}"
    );

    // A callout whose body has a list, a code block and a link: body
    // rendered and title first.
    let note_open = html
        .find("<blockquote class=\"callout note\">")
        .expect("rich-body callout: note blockquote must be present");
    let note_title = html[note_open..]
        .find("<p class=\"callout-title\">Note</p>")
        .map(|i| i + note_open)
        .expect("rich-body callout: title must be present");
    let list_pos = html[note_open..]
        .find("<li>Rich list item alpha.</li>")
        .map(|i| i + note_open);
    let code_pos = html[note_open..]
        .find("rich code block line")
        .map(|i| i + note_open);
    let link_pos = html[note_open..]
        .find("<a href=\"https://example.org/rich\">Rich example link</a>")
        .map(|i| i + note_open);
    assert!(
        html.contains("<p>Rich body intro sentence.</p>"),
        "rich-body callout: the intro paragraph must be rendered: {html}"
    );
    assert!(
        list_pos.is_some_and(|p| p > note_title),
        "rich-body callout: the list must render after the title: {html}"
    );
    assert!(
        code_pos.is_some_and(|p| p > note_title),
        "rich-body callout: the code block must render after the title: {html}"
    );
    assert!(
        link_pos.is_some_and(|p| p > note_title),
        "rich-body callout: the link must render after the title: {html}"
    );
}

#[test]
fn non_callout_block_quotes_get_no_title() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    fs::create_dir(&source).unwrap();
    write_page(&source, "not_callouts", NOT_CALLOUTS_MARKDOWN);

    let build = run_build(&source, &html_output, "html");
    assert!(
        build.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let html =
        String::from_utf8(read_file(&html_output.join("not_callouts.html"))).expect("UTF-8 HTML");

    assert!(
        !html.contains("callout-title"),
        "contract section 1: a title on the marker line, an unrecognised kind, a fold \
         marker and a plain quote are not callouts, so none may produce a \
         'callout-title' element, but one was found in not_callouts.html:\n{html}"
    );
    assert!(
        !html.contains("class=\"callout"),
        "contract section 1: none of these block quotes is a callout, so none may get a \
         'callout' class, but one was found in not_callouts.html:\n{html}"
    );

    assert!(
        html.contains("[!NOTE] Title on the marker line"),
        "`> [!NOTE] Title on the marker line` is not a callout (extra text on the marker \
         line): its literal text, unescaped ('[' and '!' are not in the HTML escaper's \
         table), must be kept in not_callouts.html:\n{html}"
    );
    assert!(
        html.contains("[!info]"),
        "`> [!info]` is not a callout (not one of the five recognised kinds): its literal \
         text must be kept, unescaped, in not_callouts.html:\n{html}"
    );
    assert!(
        html.contains("[!note]-"),
        "`> [!note]-` is not a callout (an Obsidian fold marker, not GFM): its literal \
         text must be kept, unescaped, in not_callouts.html:\n{html}"
    );
    assert!(
        html.contains("Body of a plain block quote with no marker."),
        "a plain `> ` block quote with no marker must render its text unchanged in \
         not_callouts.html:\n{html}"
    );
}

#[test]
fn stylesheet_has_callout_and_per_kind_selectors() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let html_output = temp.join("html");
    fs::create_dir(&source).unwrap();
    // A page with a callout is enough to require the stylesheet; the
    // contract says the sheet gains these rules unconditionally, but an
    // empty tree would make this test uninformative about whether the
    // renderer ever emits them.
    write_page(&source, "cases", &cases_markdown());

    let build = run_build(&source, &html_output, "html");
    assert!(
        build.status.success(),
        "HTML build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let css = String::from_utf8(read_file(&html_output.join("assets").join("style.css")))
        .expect("UTF-8 style.css");

    assert!(
        css.contains(".callout"),
        "stylesheet: expected a '.callout' selector, none found in style.css:\n{css}"
    );
    assert!(
        css.contains(".callout-title"),
        "stylesheet: expected a '.callout-title' selector, none found in style.css:\n{css}"
    );
    for (kind_lower, _label) in KINDS {
        assert!(
            has_kind_rule(&css, kind_lower),
            "stylesheet: expected a rule for kind {kind_lower:?} whose selector names both \
             'callout' and '.{kind_lower}' (accepting either the compound form \
             '.callout.{kind_lower}' or any other selector form — descendant, comma-joined, \
             etc — as long as the same selector text contains both substrings; see the \
             `has_kind_rule` comment), none found in style.css:\n{css}"
        );
    }
}

/// Accepts a CSS rule for `kind` (e.g. `note`) if some rule's selector
/// (the text before its `{`) contains both the substring `"callout"` and
/// the substring `".{kind}"`. This accepts the compound selector
/// `.callout.note` and also a descendant form like `.callout .note` or a
/// comma-joined form like `.callout, .note { ... }` bundled with a
/// `.callout` rule in the same selector list — anything naming both, in
/// one selector. It rejects an empty stylesheet and a stylesheet that is
/// missing any one kind, because for a missing kind no rule's selector
/// contains `.{kind}` at all.
fn has_kind_rule(css: &str, kind_lower: &str) -> bool {
    let dotted = format!(".{kind_lower}");
    css.split('}').any(|rule| {
        let selector = rule.split('{').next().unwrap_or("");
        selector.contains("callout") && selector.contains(&dotted)
    })
}

#[test]
fn json_output_adds_no_label_and_search_text_is_unchanged() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    let json_output = temp.join("json");
    fs::create_dir(&source).unwrap();
    write_page(&source, "json_callout", JSON_CALLOUT_MARKDOWN);

    let build = run_build(&source, &json_output, "json");
    assert!(
        build.status.success(),
        "JSON build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let page: Value = serde_json::from_slice(&read_file(&json_output.join("json_callout.json")))
        .expect("json_callout.json must parse");
    let body = page["body"]
        .as_array()
        .expect("json_callout.json body must be an array");
    let block_quote = body
        .iter()
        .find(|node| node["type"] == "block_quote")
        .unwrap_or_else(|| panic!("json_callout.json: no block_quote node in body: {page}"));

    assert_eq!(
        block_quote["kind"], "note",
        "json_callout.json: the block quote's kind must still be \"note\": {block_quote}"
    );
    let children = block_quote["children"]
        .as_array()
        .expect("json_callout.json: block_quote.children must be an array");
    assert_eq!(
        children.len(),
        1,
        "json_callout.json: contract section 3, 'no generated callout label is added': \
         the block quote's children must be exactly the source body (one paragraph), with \
         no extra title node prepended: {block_quote}"
    );
    let object = block_quote
        .as_object()
        .expect("json_callout.json: block_quote node must be a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["children", "kind", "type"],
        "json_callout.json: the block quote node must have exactly its existing members \
         {{type, kind, children}}, no added label member: {block_quote}"
    );

    let search_index: Value = serde_json::from_slice(&read_file(
        &json_output.join("assets").join("search-index.json"),
    ))
    .expect("search-index.json must parse");
    let entry = search_index["entries"]
        .as_array()
        .expect("search index has an 'entries' array")
        .iter()
        .find(|entry| entry["id"] == "json_callout")
        .unwrap_or_else(|| {
            panic!("search index has no entry for page json_callout: {search_index}")
        });
    let text = entry["text"]
        .as_str()
        .unwrap_or_else(|| panic!("json_callout search-index entry has no text field: {entry}"));
    let lower = text.to_lowercase();
    for word in LABEL_WORDS {
        assert!(
            !lower.contains(word),
            "search text for json_callout must not contain the generated label word \
             {word:?} (case-insensitively): the fixture's own source text never spells any \
             kind label outside its marker, so any occurrence is a generated label leaking \
             into search text (contract section 3): {text:?}"
        );
    }
    assert!(
        text.contains("alpha bravo charlie delta echo"),
        "search text for json_callout must still contain the callout body's own visible \
         text: {text:?}"
    );
}

#[test]
fn check_reports_zero_witnesses_on_the_callout_fixture() {
    let temp = temp_root();
    let _guard = OwnedTempDir(temp.clone());
    let source = temp.join("source");
    fs::create_dir(&source).unwrap();
    build_tree(&source);

    let check = run_check(&source);
    assert!(
        check.status.success(),
        "`rhawiki check` failed unexpectedly on the callout fixture tree: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let report: Value = serde_json::from_slice(&check.stdout).expect("check JSON must parse");
    assert_eq!(report["schema_version"], 1, "check schema_version");
    assert_eq!(
        report["pages"], 4,
        "check page count must match the fixture's four pages: {report}"
    );
    assert_eq!(
        report["witnesses"],
        Value::Array(vec![]),
        "contract section 5 (decision `callouts-check-surface`): callouts are not a \
         `rhawiki check` witness surface, so a tree with only callout and plain block \
         quote content must report zero witnesses: {report}"
    );
    assert_eq!(
        report["counts"],
        serde_json::json!({}),
        "check counts must be empty: no witness kind fired on the callout fixture: {report}"
    );
}
