#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use library::{Corpus, Digest, PageId, RelPath, Source};
use serde_json::{Map, Value};
use site::assembly::LinkTarget;
use site::assembly::{Assemble, AssembleContext, DefaultAssembler, PageModel};
use site::{Align, CalloutKind, Node, PageRenderer, Rendered};

pub const CORPUS_RELATIVE: &str = "../../xtask/tests/corpus/json-renderer";

const CASES_SHA256: &str = "fb7b1f6f9bf28d6e7eda3cc0d17b2613c6ba0070166468df89d05fc43059e854";
const REGISTRATION_SHA256: &str =
    "8d0382da61aa576fc0434143c99ad644c51296faf6fa05b1b1c8cf0de02657e9";
const SHA256SUMS_SHA256: &str = "5a48898b2705fcaefabceb3af3ebe8bb939fe1c239f997fbb2f94edc394cd697";

pub fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CORPUS_RELATIVE)
}

fn digest(bytes: &[u8]) -> String {
    Digest::of(bytes).to_string()
}

fn read(root: &Path, relative: &str) -> Vec<u8> {
    fs::read(root.join(relative)).unwrap_or_else(|error| {
        panic!(
            "cannot read fixture {}: {error}",
            root.join(relative).display()
        )
    })
}

fn collect_files(root: &Path, relative: &Path, output: &mut Vec<String>) -> Result<(), String> {
    let current = root.join(relative);
    let metadata = fs::symlink_metadata(&current)
        .map_err(|error| format!("cannot stat {}: {error}", current.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("fixture contains symlink {}", current.display()));
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(&current)
            .map_err(|error| format!("cannot read {}: {error}", current.display()))?
            .map(|entry| entry.unwrap_or_else(|error| panic!("fixture directory entry: {error}")))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let child = relative.join(entry.file_name());
            collect_files(root, &child, output)?;
        }
    } else if metadata.is_file() {
        let path = relative
            .to_str()
            .ok_or_else(|| format!("non-UTF-8 fixture path {}", current.display()))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        if RelPath::new(&path).is_err() {
            return Err(format!("unsafe fixture path {path}"));
        }
        output.push(path);
    } else {
        return Err(format!(
            "fixture contains non-file entry {}",
            current.display()
        ));
    }
    Ok(())
}

pub fn verify_fixture(root: &Path) -> Result<(), String> {
    let mut actual = Vec::new();
    collect_files(root, Path::new(""), &mut actual)?;
    actual.sort();

    let cases = read(root, "CASES.json");
    if digest(&cases) != CASES_SHA256 {
        return Err("CASES.json digest mismatch".to_owned());
    }
    let registration = read(root, "registration.toml");
    if digest(&registration) != REGISTRATION_SHA256 {
        return Err("registration.toml digest mismatch".to_owned());
    }
    let sums = read(root, "SHA256SUMS");
    if digest(&sums) != SHA256SUMS_SHA256 {
        return Err("SHA256SUMS digest mismatch".to_owned());
    }
    let mut inventory = BTreeMap::new();
    for line in std::str::from_utf8(&sums)
        .map_err(|error| format!("SHA256SUMS is not UTF-8: {error}"))?
        .lines()
    {
        let (hash, path) = line
            .split_once("  ")
            .ok_or_else(|| format!("invalid SHA256SUMS line {line:?}"))?;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!("invalid SHA256SUMS hash for {path}"));
        }
        if RelPath::new(path).is_err() || path == "registration.toml" || path == "SHA256SUMS" {
            return Err(format!("unsafe SHA256SUMS path {path}"));
        }
        if inventory.insert(path.to_owned(), hash.to_owned()).is_some() {
            return Err(format!("duplicate SHA256SUMS path {path}"));
        }
    }

    let mut expected = inventory.keys().cloned().collect::<Vec<_>>();
    expected.extend(["registration.toml".to_owned(), "SHA256SUMS".to_owned()]);
    expected.sort();
    if actual != expected {
        return Err(format!(
            "fixture inventory mismatch: {actual:?} != {expected:?}"
        ));
    }

    for (path, expected_digest) in inventory {
        let contents = read(root, &path);
        if digest(&contents) != expected_digest {
            return Err(format!("inventory payload mismatch for {path}"));
        }
    }
    Ok(())
}

pub fn load_cases(root: &Path) -> Result<Vec<Value>, String> {
    verify_fixture(root)?;
    let value: Value =
        serde_json::from_slice(&read(root, "CASES.json")).map_err(|error| error.to_string())?;
    let cases = value
        .as_array()
        .ok_or_else(|| "CASES.json root is not an array".to_owned())?
        .clone();
    validate_taxonomy(&cases)?;
    Ok(cases)
}

pub fn cases() -> Vec<Value> {
    load_cases(&corpus_root()).unwrap_or_else(|error| panic!("fixture rejected: {error}"))
}

pub fn find_case<'a>(cases: &'a [Value], id: &str) -> &'a Value {
    cases
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("missing registered case {id}"))
}

pub fn validate_taxonomy(cases: &[Value]) -> Result<(), String> {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut ids = BTreeSet::new();
    for case in cases {
        let kind = case
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| "case has no string kind".to_owned())?;
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| "case has no non-empty string id".to_owned())?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate case id {id}"));
        }
        *counts.entry(kind.to_owned()).or_default() += 1;
    }
    let expected = BTreeMap::from([
        ("cli_check".to_owned(), 1),
        ("cli_html".to_owned(), 2),
        ("cli_json".to_owned(), 1),
        ("collision".to_owned(), 1),
        ("contract".to_owned(), 1),
        ("duplicate".to_owned(), 2),
        ("project".to_owned(), 70),
        ("reconcile".to_owned(), 1),
    ]);
    if cases.len() != 79 || ids.len() != 79 || counts != expected {
        return Err(format!(
            "case taxonomy mismatch: {} {counts:?}",
            cases.len()
        ));
    }
    Ok(())
}

fn object(value: &Value) -> &Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("expected JSON object: {value}"))
}

fn exact_keys(value: &Value, expected: &[&str]) {
    let actual = object(value)
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let wanted = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, wanted, "unexpected JSON members in {value}");
}

fn string(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} is not a string in {value}"))
        .to_owned()
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    match &value[key] {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => panic!("{key} is not nullable string: {other}"),
    }
}

fn number(value: &Value, key: &str) -> u64 {
    value[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} is not an unsigned integer in {value}"))
}

fn u8_number(value: &Value, key: &str) -> u8 {
    u8::try_from(number(value, key))
        .unwrap_or_else(|_| panic!("{key} does not fit in u8 in {value}"))
}

fn usize_number(value: &Value, key: &str) -> usize {
    usize::try_from(number(value, key))
        .unwrap_or_else(|_| panic!("{key} does not fit in usize in {value}"))
}

fn callout(value: &Value) -> Option<CalloutKind> {
    match value {
        Value::Null => None,
        Value::String(kind) => Some(match kind.as_str() {
            "Note" => CalloutKind::Note,
            "Tip" => CalloutKind::Tip,
            "Important" => CalloutKind::Important,
            "Warning" => CalloutKind::Warning,
            "Caution" => CalloutKind::Caution,
            other => panic!("unknown callout {other}"),
        }),
        other => panic!("invalid callout {other}"),
    }
}

fn alignment(value: &Value) -> Align {
    match value
        .as_str()
        .unwrap_or_else(|| panic!("invalid alignment {value}"))
    {
        "None" => Align::None,
        "Left" => Align::Left,
        "Center" => Align::Center,
        "Right" => Align::Right,
        other => panic!("unknown alignment {other}"),
    }
}

pub fn node(value: &Value) -> Node {
    let kind = string(value, "kind");
    match kind.as_str() {
        "Heading" => {
            exact_keys(value, &["children", "kind", "level", "slug"]);
            Node::Heading {
                level: u8_number(value, "level"),
                slug: string(value, "slug"),
                children: value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            }
        }
        "Paragraph" => {
            exact_keys(value, &["children", "kind"]);
            Node::Paragraph(
                value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            )
        }
        "Text" => {
            exact_keys(value, &["kind", "text"]);
            Node::Text(string(value, "text"))
        }
        "Html" => {
            exact_keys(value, &["kind", "text"]);
            Node::Html(string(value, "text"))
        }
        "Code" => {
            exact_keys(value, &["kind", "text"]);
            Node::Code(string(value, "text"))
        }
        "CodeBlock" => {
            exact_keys(value, &["kind", "lang", "text"]);
            Node::CodeBlock {
                lang: optional_string(value, "lang"),
                text: string(value, "text"),
            }
        }
        "Emphasis" => {
            exact_keys(value, &["children", "kind"]);
            Node::Emphasis(
                value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            )
        }
        "Strong" => {
            exact_keys(value, &["children", "kind"]);
            Node::Strong(
                value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            )
        }
        "Strikethrough" => {
            exact_keys(value, &["children", "kind"]);
            Node::Strikethrough(
                value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            )
        }
        "Link" => {
            exact_keys(value, &["children", "href", "kind"]);
            Node::Link {
                href: string(value, "href"),
                children: value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            }
        }
        "WikiLink" => {
            exact_keys(value, &["children", "index", "kind"]);
            Node::WikiLink {
                index: usize_number(value, "index"),
                children: value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            }
        }
        "Image" => {
            exact_keys(value, &["alt", "kind", "src"]);
            Node::Image {
                src: string(value, "src"),
                alt: string(value, "alt"),
            }
        }
        "List" => {
            exact_keys(value, &["items", "kind", "start"]);
            Node::List {
                start: match &value["start"] {
                    Value::Null => None,
                    Value::Number(_) => Some(number(value, "start")),
                    other => panic!("invalid list start {other}"),
                },
                items: value["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.as_array().unwrap().iter().map(node).collect())
                    .collect(),
            }
        }
        "BlockQuote" => {
            exact_keys(value, &["callout", "children", "kind"]);
            Node::BlockQuote {
                kind: callout(&value["callout"]),
                children: value["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(node)
                    .collect(),
            }
        }
        "Table" => {
            exact_keys(value, &["align", "head", "kind", "rows"]);
            Node::Table {
                align: value["align"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(alignment)
                    .collect(),
                head: value["head"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|cell| cell.as_array().unwrap().iter().map(node).collect())
                    .collect(),
                rows: value["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| {
                        row.as_array()
                            .unwrap()
                            .iter()
                            .map(|cell| cell.as_array().unwrap().iter().map(node).collect())
                            .collect()
                    })
                    .collect(),
            }
        }
        "TaskMarker" => {
            exact_keys(value, &["checked", "kind"]);
            Node::TaskMarker(
                value["checked"]
                    .as_bool()
                    .unwrap_or_else(|| panic!("invalid task marker {value}")),
            )
        }
        "Rule" => {
            exact_keys(value, &["kind"]);
            Node::Rule
        }
        "SoftBreak" => {
            exact_keys(value, &["kind"]);
            Node::SoftBreak
        }
        "HardBreak" => {
            exact_keys(value, &["kind"]);
            Node::HardBreak
        }
        other => panic!("unknown source node kind {other}"),
    }
}

pub fn page_model(value: &Value) -> PageModel {
    exact_keys(
        value,
        &[
            "backlinks",
            "body",
            "breadcrumbs",
            "built_at",
            "id",
            "links",
            "title",
            "toc",
        ],
    );
    let links = value["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|link| match string(link, "kind").as_str() {
            "Page" => {
                exact_keys(link, &["anchor", "id", "kind"]);
                LinkTarget::Page {
                    id: PageId::new(&string(link, "id")),
                    anchor: optional_string(link, "anchor"),
                }
            }
            "Unresolved" => {
                exact_keys(link, &["kind"]);
                LinkTarget::Unresolved
            }
            other => panic!("unknown link kind {other}"),
        })
        .collect();
    PageModel {
        id: PageId::new(&string(value, "id")),
        title: string(value, "title"),
        toc: value["toc"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                exact_keys(entry, &["anchor", "level", "text"]);
                site::assembly::TocEntry {
                    level: u8_number(entry, "level"),
                    text: string(entry, "text"),
                    anchor: string(entry, "anchor"),
                }
            })
            .collect(),
        breadcrumbs: value["breadcrumbs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item.as_str().unwrap().to_owned())
            .collect(),
        backlinks: value["backlinks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                exact_keys(item, &["id", "title"]);
                (PageId::new(&string(item, "id")), string(item, "title"))
            })
            .collect(),
        links,
        body: value["body"].as_array().unwrap().iter().map(node).collect(),
        built_at: optional_string(value, "built_at"),
    }
}

pub fn models_from_values(values: &Value) -> Vec<PageModel> {
    values
        .as_array()
        .unwrap_or_else(|| panic!("pages is not an array"))
        .iter()
        .map(page_model)
        .collect()
}

pub fn corpus_from_sources(values: &Value) -> Corpus {
    let sources = values
        .as_array()
        .unwrap()
        .iter()
        .map(|source| {
            exact_keys(source, &["path", "text"]);
            let path = RelPath::new(&string(source, "path")).unwrap();
            Source::new(path, string(source, "text"))
        })
        .collect();
    Corpus::new(sources).unwrap()
}

pub fn models_from_corpus(corpus: &Corpus, built_at: &str) -> Vec<PageModel> {
    let (documents, graph) = site::analyse(corpus);
    let titles = documents
        .iter()
        .map(|document| (document.id.clone(), document.title.clone()))
        .collect::<BTreeMap<_, _>>();
    let lookup = |id: &PageId| titles.get(id).cloned();
    let context = AssembleContext {
        built_at: Some(built_at.to_owned()),
    };
    documents
        .iter()
        .map(|document| DefaultAssembler.assemble(document, &graph, &lookup, &context))
        .collect()
}

fn assert_json_bytes(rendered: &Rendered, expected_path: &str, expected: &Value) {
    assert_eq!(rendered.path.as_str(), expected_path);
    let text = std::str::from_utf8(&rendered.bytes).expect("renderer output must be UTF-8");
    assert!(
        !text.contains('\r'),
        "renderer output must use LF line endings"
    );
    assert!(text.ends_with('\n'), "renderer output must end in newline");
    assert!(
        text.ends_with("}\n"),
        "renderer output must end with exactly one newline after the JSON object"
    );
    assert!(
        !text.ends_with("\n\n"),
        "renderer output has multiple trailing newlines"
    );
    assert!(
        text.lines().count() > 1,
        "renderer output must be pretty JSON"
    );
    let actual: Value = serde_json::from_str(text).expect("renderer output must be JSON");
    assert!(
        actual.is_object(),
        "renderer output root must be a JSON object"
    );
    assert_eq!(
        actual, *expected,
        "decoded output mismatch at {expected_path}"
    );
}

fn expected_object<'a>(value: &'a Value, key: &str) -> &'a Map<String, Value> {
    value[key]
        .as_object()
        .unwrap_or_else(|| panic!("{key} is not an object"))
}

pub fn assert_project_case(case: &Value) {
    let has_comparison = case.get("comparison_pages").is_some();
    if has_comparison {
        let mut allowed = vec![
            "comparison_pages",
            "expected_comparison_index",
            "expected_index",
            "expected_pages",
            "id",
            "kind",
            "pages",
        ];
        if case.get("expected_comparison_pages").is_some() {
            allowed.push("expected_comparison_pages");
        }
        exact_keys(case, &allowed);
        assert!(case.get("expected_comparison_index").is_some());
    } else {
        exact_keys(
            case,
            &["expected_index", "expected_pages", "id", "kind", "pages"],
        );
    }
    let models = models_from_values(&case["pages"]);
    let snapshot = models.clone();
    let renderer = adapter_json::JsonRenderer::new(&models).unwrap();
    let independent_renderer = adapter_json::JsonRenderer::new(&models).unwrap();
    assert_eq!(models, snapshot, "renderer mutated its input models");

    let expected_pages = expected_object(case, "expected_pages");
    let expected_paths = expected_pages.keys().cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for model in &models {
        let first = renderer.render(model);
        let second = renderer.render(model);
        assert_eq!(first, second, "repeated page render differs");
        let independent = independent_renderer.render(model);
        assert_eq!(
            first, independent,
            "equal-input page render differs across renderer instances"
        );
        let path = first.path.as_str().to_owned();
        assert!(seen.insert(path.clone()), "duplicate page output {path}");
        assert_json_bytes(&first, &path, &expected_pages[&path]);
    }
    assert_eq!(seen, expected_paths);

    let assets = renderer.assets();
    let repeated_assets = renderer.assets();
    let independent_assets = independent_renderer.assets();
    assert_eq!(assets.len(), 1, "renderer must return exactly one asset");
    assert_eq!(assets, repeated_assets, "repeated assets differ");
    assert_eq!(
        assets, independent_assets,
        "equal-input assets differ across renderer instances"
    );
    assert_eq!(assets[0].path.as_str(), "assets/search-index.json");
    assert_json_bytes(
        &assets[0],
        "assets/search-index.json",
        &case["expected_index"],
    );

    if has_comparison {
        let comparison = models_from_values(&case["comparison_pages"]);
        let comparison_renderer = adapter_json::JsonRenderer::new(&comparison).unwrap();
        let expected = case
            .get("expected_comparison_pages")
            .map(|_| expected_object(case, "expected_comparison_pages"));
        let mut comparison_paths = BTreeSet::new();
        for model in &comparison {
            let rendered = comparison_renderer.render(model);
            let repeated = comparison_renderer.render(model);
            assert_eq!(rendered, repeated, "repeated comparison render differs");
            let path = rendered.path.as_str().to_owned();
            assert!(comparison_paths.insert(path.clone()));
            if let Some(expected) = expected {
                assert_json_bytes(&rendered, &path, &expected[&path]);
            }
        }
        if let Some(expected) = expected {
            let expected_paths = expected.keys().cloned().collect::<BTreeSet<_>>();
            assert_eq!(comparison_paths, expected_paths);
        }
        let comparison_assets = comparison_renderer.assets();
        assert_eq!(comparison_assets.len(), 1);
        assert_eq!(comparison_assets[0].bytes, assets[0].bytes);
        assert_eq!(comparison_assets[0], assets[0]);
        assert_json_bytes(
            &comparison_assets[0],
            "assets/search-index.json",
            &case["expected_comparison_index"],
        );
    }
}

pub fn index_ids(bytes: &[u8]) -> Vec<String> {
    let value: Value = serde_json::from_slice(bytes).unwrap();
    value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| string(entry, "id"))
        .collect()
}

pub fn sink_paths(sink: &site::testing::RecordingSink) -> Vec<String> {
    sink.files
        .keys()
        .map(|path| path.as_str().to_owned())
        .collect()
}
