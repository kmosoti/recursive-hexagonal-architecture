use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use serde_json::{Value, json};
use xtask::modules::extract::{Edge, Extracted, extract};

#[derive(Debug, Deserialize)]
struct RandomCase {
    id: String,
    crate_name: String,
    edition: String,
    files: BTreeMap<String, String>,
}

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        for _ in 0..128 {
            let serial = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rha-m2-module-extraction-{label}-{}-{serial}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!(
                    "failed to create scratch directory {}: {error}",
                    path.display()
                ),
            }
        }

        panic!("could not allocate a unique scratch directory");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path)
            && self.path.exists()
        {
            eprintln!(
                "failed to remove scratch directory {}: {error}",
                self.path.display()
            );
        }
    }
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/module")
}

fn validate_relative(path: &str) -> Result<&Path, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() {
        return Err("empty relative path".to_owned());
    }
    if path.is_absolute() {
        return Err(format!("absolute path is not allowed: {}", path.display()));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(format!(
                "path traversal or non-normal component: {}",
                path.display()
            ));
        }
    }
    Ok(path)
}

fn write_files(root: &Path, files: &[(&str, &str)]) -> Result<(), String> {
    let mut checked = Vec::with_capacity(files.len());
    for &(relative, contents) in files {
        checked.push((
            validate_relative(relative)
                .map_err(|error| format!("{relative}: {error}"))?
                .to_path_buf(),
            contents,
        ));
    }

    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    for (relative, contents) in checked {
        let target = root.join(relative);
        if !target.starts_with(root) {
            return Err(format!(
                "materialized path escaped root: {}",
                target.display()
            ));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(&target, contents).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(())
}

fn materialize_random_case(root: &Path, files: &BTreeMap<String, String>) -> Result<(), String> {
    let mut checked = Vec::with_capacity(files.len());
    for (relative, contents) in files {
        checked.push((
            validate_relative(relative)
                .map_err(|error| format!("{relative}: {error}"))?
                .to_path_buf(),
            contents.as_str(),
        ));
    }

    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    for (relative, contents) in checked {
        let target = root.join(relative);
        if !target.starts_with(root) {
            return Err(format!(
                "materialized path escaped root: {}",
                target.display()
            ));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(&target, contents).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(())
}

fn extract_text_case(case_id: &str, files: &[(&str, &str)]) -> Result<Extracted, String> {
    let scratch = Scratch::new(case_id);
    let crate_root = scratch.path().join("crate");
    write_files(&crate_root, files).map_err(|error| format!("{case_id}: {error}"))?;
    let root_file = crate_root.join("src/lib.rs");
    extract(
        &crate_root,
        &root_file,
        "control_crate",
        "2021",
        &BTreeSet::new(),
    )
    .map_err(|error| format!("{case_id}: {error}"))
}

fn registered_count(registration: &toml::Value, key: &str) -> usize {
    registration
        .get("counts")
        .and_then(toml::Value::as_table)
        .and_then(|counts| counts.get(key))
        .and_then(toml::Value::as_integer)
        .unwrap_or_else(|| panic!("registration is missing counts.{key}")) as usize
}

fn is_random_case(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    let bytes = stem.as_bytes();
    bytes.len() == 5 && bytes[0] == b'R' && bytes[1..].iter().all(|byte| byte.is_ascii_digit())
}

fn first_array_difference(actual: &Value, expected: &Value, field: &str) -> Option<String> {
    let actual_items = actual.get(field)?.as_array()?;
    let expected_items = expected.get(field)?.as_array()?;

    if let Some(item) = expected_items
        .iter()
        .find(|item| !actual_items.contains(item))
    {
        return Some(format!("missing {field}: {item}"));
    }
    if let Some(item) = actual_items
        .iter()
        .find(|item| !expected_items.contains(item))
    {
        return Some(format!("extra {field}: {item}"));
    }
    None
}

#[test]
fn random_corpus_matches_registered_reference_outputs() {
    let root = corpus_root();
    let registration: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("registration.toml")).expect("module registration"),
    )
    .expect("valid module registration");

    let mut cases = fs::read_dir(root.join("random"))
        .expect("random corpus directory")
        .map(|entry| entry.expect("random corpus entry").path())
        .filter(|path| is_random_case(path))
        .collect::<Vec<_>>();
    cases.sort();

    let registered_cases = registered_count(&registration, "random_cases");
    assert_eq!(
        registered_cases, 256,
        "registration must retain the decided corpus size"
    );
    assert_eq!(cases.len(), registered_cases, "random corpus case count");

    let mut failures = Vec::new();
    let mut total_edges = 0;
    let mut total_test_edges = 0;
    let mut total_heuristic_edges = 0;
    let mut total_limitations = 0;

    for case_path in cases {
        let case_id = case_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("random case id");
        let case: RandomCase = serde_json::from_slice(
            &fs::read(&case_path).unwrap_or_else(|error| panic!("{case_id}: {error}")),
        )
        .unwrap_or_else(|error| panic!("{case_id}: invalid random case: {error}"));
        assert_eq!(case.id, case_id, "registered case identity");

        let scratch = Scratch::new(case_id);
        let crate_root = scratch.path().join("crate");
        if let Err(error) = materialize_random_case(&crate_root, &case.files) {
            failures.push(format!("{case_id}: materialization failed: {error}"));
            continue;
        }

        let extracted = extract(
            &crate_root,
            &crate_root.join("src/lib.rs"),
            &case.crate_name,
            &case.edition,
            &BTreeSet::new(),
        );

        let expected_path = case_path.with_file_name(format!("{case_id}.expected.json"));
        let expected: Value = serde_json::from_slice(
            &fs::read(&expected_path)
                .unwrap_or_else(|error| panic!("{case_id}: expected output: {error}")),
        )
        .unwrap_or_else(|error| panic!("{case_id}: invalid expected output: {error}"));

        match extracted {
            Err(error) => failures.push(format!("{case_id}: extractor returned Err: {error}")),
            Ok(got) => {
                total_edges += got.edges.len();
                total_test_edges += got.edges.iter().filter(|edge| edge.test_only).count();
                total_heuristic_edges += got
                    .edges
                    .iter()
                    .filter(|edge| edge.extraction == "heuristic")
                    .count();
                total_limitations += got.limitations.len();

                let actual = json!({
                    "schema_version": 1,
                    "edges": &got.edges,
                    "limitations": &got.limitations,
                });
                if actual != expected {
                    let mut details = Vec::new();
                    for field in ["edges", "limitations"] {
                        if let Some(detail) = first_array_difference(&actual, &expected, field) {
                            details.push(detail);
                        }
                    }
                    if details.is_empty() {
                        details.push(format!("actual={actual}, expected={expected}"));
                    }
                    failures.push(format!("{case_id}: {}", details.join("; ")));
                }
            }
        }
    }

    println!(
        "module extraction corpus: cases={} normalized_edges={} test_edges={} heuristic_edges={} limitations={}",
        registered_cases, total_edges, total_test_edges, total_heuristic_edges, total_limitations
    );

    assert_eq!(
        total_edges,
        registered_count(&registration, "random_edges"),
        "registered random edge count"
    );
    assert_eq!(
        total_test_edges,
        registered_count(&registration, "random_test_edges"),
        "registered random test-edge count"
    );
    assert_eq!(
        total_heuristic_edges,
        registered_count(&registration, "random_heuristic_edges"),
        "registered random heuristic-edge count"
    );
    assert_eq!(
        total_limitations,
        registered_count(&registration, "random_limitations"),
        "registered random limitation count"
    );
    assert!(
        failures.is_empty(),
        "random module extraction mismatches:\n{}",
        failures.join("\n")
    );
}

fn assert_edge(
    extracted: &Extracted,
    case_id: &str,
    source: &str,
    target: &str,
    test_only: bool,
    extraction: &str,
) {
    let expected = Edge {
        source: source.to_owned(),
        target: target.to_owned(),
        test_only,
        extraction: extraction.to_owned(),
    };
    assert!(
        extracted.edges.contains(&expected),
        "{case_id}: missing edge {expected:?}; actual edges: {:?}",
        extracted.edges
    );
}

#[test]
fn cfg_and_path_controls_preserve_source_level_semantics() {
    let extracted = extract_text_case(
        "controls-cfg",
        &[(
            "src/lib.rs",
            r#"
pub mod a { pub struct Item; }
pub mod b {
    pub struct Type;
    impl Type { pub fn method() {} }
}
pub mod production {
    use crate::a::Item;
}
#[cfg(test)]
pub mod nested_tests {
    use crate::a::Item;
}
pub mod function_cfg {
    pub fn production(_: crate::b::Type) {}
    #[cfg(test)]
    pub fn only_in_tests(_: crate::a::Item) {}
}
pub mod local_use {
    pub fn unused() {
        use crate::a::Item;
    }
}
pub mod facade_user {
    pub fn call() {
        crate::b::Type::method();
    }
}
"#,
        )],
    )
    .expect("CONTROL-CFG: valid source should extract");

    assert_edge(
        &extracted,
        "CONTROL-CFG",
        "control_crate::production",
        "control_crate::a::Item",
        false,
        "syntax",
    );
    assert_edge(
        &extracted,
        "CONTROL-CFG",
        "control_crate::nested_tests",
        "control_crate::a::Item",
        true,
        "syntax",
    );
    assert_edge(
        &extracted,
        "CONTROL-CFG",
        "control_crate::function_cfg",
        "control_crate::b::Type",
        false,
        "syntax",
    );
    assert_edge(
        &extracted,
        "CONTROL-CFG",
        "control_crate::function_cfg",
        "control_crate::a::Item",
        true,
        "syntax",
    );
    assert_edge(
        &extracted,
        "CONTROL-CFG",
        "control_crate::local_use",
        "control_crate::a::Item",
        false,
        "syntax",
    );
    assert_edge(
        &extracted,
        "CONTROL-CFG",
        "control_crate::facade_user",
        "control_crate::b::Type::method",
        false,
        "syntax",
    );
    assert!(
        !extracted.limitations.iter().any(|limitation| {
            limitation.source == "control_crate::facade_user"
                && limitation.code == "unsupported_associated_path"
        }),
        "CONTROL-CFG: associated item path became a limitation: {:?}",
        extracted.limitations
    );

    let malformed = extract_text_case("controls-malformed", &[("src/lib.rs", "pub mod {")]);
    assert!(
        malformed.is_err(),
        "CONTROL-MALFORMED: malformed Rust must return Err, got {malformed:?}"
    );
}

#[test]
fn source_boundaries_and_recursive_sources_are_safe() {
    let scratch = Scratch::new("controls-boundary");
    let crate_root = scratch.path().join("crate");
    write_files(
        scratch.path(),
        &[("outside.rs", "this is syntactically invalid Rust !!!")],
    )
    .expect("CONTROL-PATH-ESCAPE: outside fixture");
    write_files(
        &crate_root,
        &[("src/lib.rs", r#"#[path = "../../outside.rs"] mod escaped;"#)],
    )
    .expect("CONTROL-PATH-ESCAPE: crate fixture");
    let escaped = extract(
        &crate_root,
        &crate_root.join("src/lib.rs"),
        "control_crate",
        "2021",
        &BTreeSet::new(),
    );
    let error = escaped.expect_err("CONTROL-PATH-ESCAPE: expected boundary refusal");
    assert!(
        error.contains("outside crate root"),
        "CONTROL-PATH-ESCAPE: error did not identify the boundary: {error}"
    );

    let missing = extract_text_case("controls-missing", &[("src/lib.rs", "mod missing;")]);
    match missing {
        Err(_) => {}
        Ok(extracted) => assert!(
            !extracted.limitations.is_empty(),
            "CONTROL-MISSING: missing source returned Ok without a limitation"
        ),
    }

    let duplicated_source = extract_text_case(
        "controls-shared-source",
        &[
            (
                "src/lib.rs",
                r#"
pub mod dependency { pub struct Item; }
#[path = "shared.rs"] pub mod first;
#[path = "shared.rs"] pub mod second;
"#,
            ),
            (
                "src/shared.rs",
                "pub fn takes(_: crate::dependency::Item) {}",
            ),
        ],
    )
    .expect("CONTROL-SHARED-SOURCE: valid source should extract");
    assert!(
        duplicated_source
            .modules
            .contains_key("control_crate::first"),
        "CONTROL-SHARED-SOURCE: first logical module was not loaded"
    );
    assert!(
        duplicated_source
            .modules
            .contains_key("control_crate::second"),
        "CONTROL-SHARED-SOURCE: second logical module was not loaded"
    );
    assert_edge(
        &duplicated_source,
        "CONTROL-SHARED-SOURCE",
        "control_crate::first",
        "control_crate::dependency::Item",
        false,
        "syntax",
    );
    assert_edge(
        &duplicated_source,
        "CONTROL-SHARED-SOURCE",
        "control_crate::second",
        "control_crate::dependency::Item",
        false,
        "syntax",
    );

    let cyclic = extract_text_case(
        "controls-cycle",
        &[
            ("src/lib.rs", "mod a;"),
            ("src/a.rs", r#"#[path = "../b.rs"] mod b;"#),
            ("src/b.rs", r#"#[path = "../a.rs"] mod a_again;"#),
        ],
    );
    match cyclic {
        Err(_) => {}
        Ok(extracted) => assert!(
            !extracted.limitations.is_empty(),
            "CONTROL-CYCLE: cyclic source declarations returned Ok without a limitation"
        ),
    }
}

#[cfg(unix)]
#[test]
fn symlinked_module_source_cannot_escape_crate_root() {
    use std::os::unix::fs::symlink;

    let scratch = Scratch::new("controls-symlink");
    let crate_root = scratch.path().join("crate");
    let outside = scratch.path().join("outside.rs");
    write_files(
        scratch.path(),
        &[("outside.rs", "this is syntactically invalid Rust !!!")],
    )
    .expect("CONTROL-SYMLINK: outside fixture");
    write_files(
        &crate_root,
        &[("src/lib.rs", r#"#[path = "link.rs"] mod linked;"#)],
    )
    .expect("CONTROL-SYMLINK: crate fixture");
    symlink(&outside, crate_root.join("src/link.rs")).expect("CONTROL-SYMLINK: create symlink");

    let result = extract(
        &crate_root,
        &crate_root.join("src/lib.rs"),
        "control_crate",
        "2021",
        &BTreeSet::new(),
    );
    let error = result.expect_err("CONTROL-SYMLINK: expected boundary refusal");
    assert!(
        error.contains("outside crate root"),
        "CONTROL-SYMLINK: error did not identify the boundary: {error}"
    );
}

#[test]
fn macro_argument_groups_and_trailing_tokens_are_heuristic_only() {
    let extracted = extract_text_case(
        "controls-macro",
        &[(
            "src/lib.rs",
            r#"
pub mod a { pub struct Item; }
pub mod b { pub struct Item; }
pub mod caller {
    macro_rules! accept {
        ($($tokens:tt)*) => {};
    }
    pub fn invoke() {
        accept!((crate::a::Item), [crate::b::Item::]);
    }
}

"#,
        )],
    )
    .expect("CONTROL-MACRO: token-stream source should extract");

    let caller_edges = extracted
        .edges
        .iter()
        .filter(|edge| edge.source == "control_crate::caller")
        .collect::<Vec<_>>();
    let actual_targets = caller_edges
        .iter()
        .map(|edge| edge.target.clone())
        .collect::<BTreeSet<_>>();
    let expected_targets = [
        "control_crate::a::Item".to_owned(),
        "control_crate::b::Item".to_owned(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    assert_eq!(
        actual_targets, expected_targets,
        "CONTROL-MACRO: argument findings were not limited to the qualified argument paths: {caller_edges:?}"
    );
    assert!(
        caller_edges
            .iter()
            .all(|edge| !edge.test_only && edge.extraction == "heuristic"),
        "CONTROL-MACRO: argument edges were not all heuristic production edges: {caller_edges:?}"
    );
}

#[test]
fn path_attributes_follow_measured_rust_source_directories() {
    let layouts = [
        (
            "normal-file",
            "mod a;",
            "src/a.rs",
            "mod child;",
            "src/a/child.rs",
            "a::child",
        ),
        (
            "file-path",
            "mod a;",
            "src/a.rs",
            "#[path=\"mapped.rs\"] mod child;",
            "src/mapped.rs",
            "a::child",
        ),
        (
            "mapped-parent",
            "#[path=\"mapped.rs\"] mod a;",
            "src/mapped.rs",
            "mod child;",
            "src/child.rs",
            "a::child",
        ),
        (
            "inline-path",
            "#[path=\"chosen\"] mod a { mod child; }",
            "src/unused.rs",
            "",
            "src/chosen/child.rs",
            "a::child",
        ),
        (
            "nested-inline-path",
            "mod a;",
            "src/a.rs",
            "#[path=\"chosen\"] mod inside { mod child; }",
            "src/chosen/child.rs",
            "a::inside::child",
        ),
    ];
    for (id, root, file, body, child_file, source) in layouts {
        let root = format!("pub mod peer {{ pub struct Token; }} {root}");
        let got = extract_text_case(
            id,
            &[
                ("src/lib.rs", &root),
                (file, body),
                (child_file, "use crate::peer::Token;"),
            ],
        )
        .expect(id);
        assert!(got.limitations.is_empty(), "{id}: {:?}", got.limitations);
        assert_eq!(
            got.edges,
            vec![Edge {
                source: format!("control_crate::{source}"),
                target: "control_crate::peer::Token".into(),
                test_only: false,
                extraction: "syntax".into(),
            }],
            "{id}"
        );
    }
}

#[test]
fn glob_reexports_preserve_the_facade_and_visibility_is_not_a_dependency() {
    let got = extract_text_case(
        "facade",
        &[(
            "src/lib.rs",
            r#"
pub mod underlying { pub struct Thing; }
pub mod facade { use crate::underlying as provider; pub use provider::*; }
pub mod caller {
    use crate::facade::*;
    pub(in crate) fn take(_: Thing) {}
    pub fn direct(_: crate::facade::Thing) {}
}
"#,
        )],
    )
    .expect("facade source");
    assert!(got.limitations.is_empty(), "{:?}", got.limitations);
    let targets: BTreeSet<_> = got
        .edges
        .iter()
        .filter(|edge| edge.source == "control_crate::caller")
        .map(|edge| edge.target.as_str())
        .collect();
    assert_eq!(
        targets,
        BTreeSet::from(["control_crate::facade", "control_crate::facade::Thing"])
    );
}

#[test]
fn lexical_pattern_bindings_are_not_module_references() {
    for (id, body) in [
        ("closure", "let _f = |ordering: u8| ordering;"),
        ("for", "for ordering in [1u8] { let _ = ordering; }"),
        (
            "match",
            "match Some(1u8) { Some(ordering) if ordering > 0 => { let _ = ordering; }, _ => {} }",
        ),
        (
            "if-let",
            "if let Some(ordering) = Some(1u8) { let _ = ordering; }",
        ),
        (
            "while-let",
            "while let Some(ordering) = Some(1u8) { let _ = ordering; break; }",
        ),
    ] {
        let source =
            format!("pub mod caller {{ pub mod ordering {{}} pub fn run() {{ {body} }} }}");
        let got = extract_text_case(id, &[("src/lib.rs", &source)]).expect(id);
        assert!(got.edges.is_empty(), "{id}: {:?}", got.edges);
    }
}
