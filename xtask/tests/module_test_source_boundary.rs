use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;
use xtask::modules::extract::{Extracted, extract, extract_with_test_root};
use xtask::modules::workspace::{source_files, source_files_with_test_root};
use xtask::util::sha256_hex;

const PAYLOADS: [&str; 7] = [
    "CASES.json",
    "reference.py",
    "README.md",
    "selftestreport.txt",
    "contractaddendum.md",
    "originalcontract.md",
    "prompt.md",
];

const PACKAGE_FILES: [&str; 9] = [
    "CASES.json",
    "README.md",
    "SHA256SUMS",
    "contractaddendum.md",
    "originalcontract.md",
    "prompt.md",
    "reference.py",
    "registration.toml",
    "selftestreport.txt",
];

const CASE_IDS: [&str; 12] = [
    "strict_external_cfg_test_helper",
    "workspace_external_cfg_test_helper",
    "workspace_production_external_helper",
    "nested_out_of_line_inherited_cfg_test_helper",
    "workspace_cfg_test_target_outside_workspace",
    "workspace_cfg_test_canonical_symlink_inside",
    "workspace_cfg_test_canonical_symlink_outside",
    "workspace_production_symlink_inside_workspace",
    "workspace_external_inner_cfg_test_non_test_declaration",
    "workspace_external_nonexact_cfg_test_declaration",
    "workspace_external_cfg_attr_path_declaration",
    "workspace_root_narrower_than_crate_root",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    kind: String,
    mode: String,
    crate_name: String,
    crate_root: String,
    workspace_root: String,
    root_file: String,
    files: BTreeMap<String, String>,
    symlinks: BTreeMap<String, String>,
    expected: Expected,
    mutation: Option<Mutation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    outcome: String,
    reason_contains: Option<String>,
    modules: Vec<String>,
    test_edge: Option<TestEdge>,
    source_files: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TestEdge {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mutation {
    path: String,
    text: String,
    expected_sha256: String,
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
                "rha-m2-module-test-boundary-{label}-{}-{serial}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("creating {}: {error}", path.display()),
            }
        }

        panic!("could not allocate an owned scratch directory");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask workspace root")
        .to_path_buf()
}

fn package_root() -> PathBuf {
    root().join("xtask/tests/corpus/module-test-boundary")
}

fn sha256(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

fn exact_keys(actual: &BTreeSet<String>, expected: &[&str]) -> Result<(), String> {
    let expected = expected
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    if actual != &expected {
        return Err(format!("keys differ: actual={actual:?}, expected={expected:?}"));
    }
    Ok(())
}

fn verify_package(path: &Path) -> Result<Vec<Case>, String> {
    let actual_files = fs::read_dir(path)
        .map_err(|error| format!("reading package directory {}: {error}", path.display()))?
        .map(|entry| {
            let entry = entry.map_err(|error| format!("reading package entry: {error}"))?;
            if !entry
                .file_type()
                .map_err(|error| format!("reading package entry type: {error}"))?
                .is_file()
            {
                return Err(format!("package entry is not a file: {}", entry.path().display()));
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| "package filename is not UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let actual_files = actual_files.into_iter().collect::<BTreeSet<_>>();
    exact_keys(
        &actual_files,
        &[
            "CASES.json",
            "README.md",
            "SHA256SUMS",
            "contractaddendum.md",
            "originalcontract.md",
            "prompt.md",
            "reference.py",
            "registration.toml",
            "selftestreport.txt",
        ],
    )?;

    let sums_bytes = fs::read(path.join("SHA256SUMS"))
        .map_err(|error| format!("reading SHA256SUMS: {error}"))?;
    let sums = String::from_utf8(sums_bytes.clone())
        .map_err(|error| format!("SHA256SUMS is not UTF-8: {error}"))?;
    if !sums.ends_with('\n') {
        return Err("SHA256SUMS must end with LF".to_owned());
    }

    let mut listed = BTreeMap::new();
    for line in sums[..sums.len() - 1].split('\n') {
        if line.ends_with('\r') {
            return Err("SHA256SUMS contains CR".to_owned());
        }
        let (digest, relative) = line
            .split_once("  ")
            .ok_or_else(|| format!("malformed SHA256SUMS line: {line:?}"))?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(format!("invalid SHA256SUMS digest: {digest}"));
        }
        if !PAYLOADS.contains(&relative) {
            return Err(format!("unexpected SHA256SUMS path: {relative}"));
        }
        if listed
            .insert(relative.to_owned(), digest.to_owned())
            .is_some()
        {
            return Err(format!("duplicate SHA256SUMS path: {relative}"));
        }
    }
    let listed_names = listed.keys().cloned().collect::<BTreeSet<_>>();
    exact_keys(
        &listed_names,
        &[
            "CASES.json",
            "README.md",
            "contractaddendum.md",
            "originalcontract.md",
            "prompt.md",
            "reference.py",
            "selftestreport.txt",
        ],
    )?;
    for (relative, expected) in listed {
        let bytes = fs::read(path.join(&relative))
            .map_err(|error| format!("reading payload {relative}: {error}"))?;
        if sha256(&bytes) != expected {
            return Err(format!("{relative} digest is not the registered digest"));
        }
    }

    let registration_text = fs::read_to_string(path.join("registration.toml"))
        .map_err(|error| format!("reading registration.toml: {error}"))?;
    let registration: toml::Value = toml::from_str(&registration_text)
        .map_err(|error| format!("parsing registration.toml: {error}"))?;
    let registration_table = registration
        .as_table()
        .ok_or_else(|| "registration.toml root is not a table".to_owned())?;
    let registration_keys = registration_table.keys().cloned().collect::<BTreeSet<_>>();
    exact_keys(
        &registration_keys,
        &[
            "actual_case_count",
            "authored",
            "content_sha256",
            "effort",
            "files",
            "id",
            "kind",
            "kind_counts",
            "model",
            "payloads",
            "prng",
            "schema_version",
            "source_snapshots",
        ],
    )?;
    if registration["schema_version"].as_integer() != Some(1)
        || registration["id"].as_str() != Some("module-test-boundary")
        || registration["kind"].as_str() != Some("module_test_boundary")
        || registration["payloads"].as_integer() != Some(7)
        || registration["files"].as_integer() != Some(9)
        || registration["actual_case_count"].as_integer() != Some(12)
        || registration["authored"].as_bool() != Some(true)
        || registration["prng"].as_bool() != Some(false)
        || registration["model"].as_str() != Some("gpt-5.6-luna")
        || registration["effort"].as_str() != Some("high")
        || registration["content_sha256"].as_str() != Some(sha256(&sums_bytes).as_str())
    {
        return Err("registration metadata mismatch".to_owned());
    }
    if registration["kind_counts"]["module_test_boundary"].as_integer() != Some(12) {
        return Err("registration kind count mismatch".to_owned());
    }

    let snapshots = registration["source_snapshots"]
        .as_array()
        .ok_or_else(|| "source_snapshots is not an array".to_owned())?;
    if snapshots.len() != 3 {
        return Err(format!("expected three source snapshots, found {}", snapshots.len()));
    }
    let snapshot_spec = [
        (
            "contractaddendum.md",
            "docs/architecture/module-test-source-boundary.md",
        ),
        (
            "originalcontract.md",
            "docs/architecture/module-check-contract.md",
        ),
        (
            "prompt.md",
            "xtask/tests/corpus/write-prompts/CHG-008/pd-module-test-boundary-generator-prompt.md",
        ),
    ];
    for (snapshot, (relative, original)) in snapshots.iter().zip(snapshot_spec) {
        let table = snapshot
            .as_table()
            .ok_or_else(|| "source snapshot is not a table".to_owned())?;
        let keys = table.keys().cloned().collect::<BTreeSet<_>>();
        exact_keys(&keys, &["original_documentary_path", "relative_path", "sha256"])?;
        let bytes = fs::read(path.join(relative))
            .map_err(|error| format!("reading source snapshot {relative}: {error}"))?;
        if table["relative_path"].as_str() != Some(relative)
            || table["original_documentary_path"].as_str() != Some(original)
            || table["sha256"].as_str() != Some(sha256(&bytes).as_str())
        {
            return Err(format!("source snapshot metadata mismatch: {relative}"));
        }
    }

    let cases_bytes =
        fs::read(path.join("CASES.json")).map_err(|error| format!("reading CASES.json: {error}"))?;
    let cases: Vec<Case> = serde_json::from_slice(&cases_bytes)
        .map_err(|error| format!("parsing CASES.json: {error}"))?;
    if cases.len() != 12 {
        return Err(format!("expected twelve cases, found {}", cases.len()));
    }
    Ok(cases)
}

fn copy_package(source: &Path, destination: &Path) {
    fs::create_dir(destination).expect("copied package directory");
    for relative in PACKAGE_FILES {
        fs::copy(source.join(relative), destination.join(relative))
            .unwrap_or_else(|error| panic!("copying {relative}: {error}"));
    }
}

fn normal_parts(value: &str) -> Vec<String> {
    assert!(!value.is_empty(), "empty virtual path");
    assert!(!value.contains('\\'), "backslash in virtual path: {value}");
    let path = Path::new(value);
    assert!(path.is_relative(), "absolute virtual path: {value}");
    path.components()
        .map(|component| match component {
            Component::Normal(value) => value
                .to_str()
                .unwrap_or_else(|| panic!("non-UTF-8 virtual path: {value:?}"))
                .to_owned(),
            other => panic!("non-normal virtual path component {other:?}: {value}"),
        })
        .collect()
}

fn resolve_virtual_link(link: &str, target: &str) -> String {
    assert!(!target.is_empty(), "empty symlink target");
    assert!(!target.contains('\\'), "backslash in symlink target: {target}");
    assert!(Path::new(target).is_relative(), "absolute symlink target: {target}");

    let mut parts = normal_parts(link);
    parts.pop().expect("symlink has a parent");
    for component in Path::new(target).components() {
        match component {
            Component::Normal(value) => parts.push(
                value
                    .to_str()
                    .unwrap_or_else(|| panic!("non-UTF-8 symlink target: {target}"))
                    .to_owned(),
            ),
            Component::ParentDir => {
                assert!(parts.pop().is_some(), "symlink target escapes case root: {target}");
            }
            Component::CurDir => {}
            other => panic!("invalid symlink target component {other:?}: {target}"),
        }
    }
    assert!(!parts.is_empty(), "symlink target resolves to case root");
    parts.join("/")
}

fn materialize_case(case: &Case, owned_root: &Path) {
    let file_paths = case
        .files
        .keys()
        .map(|path| {
            let parts = normal_parts(path);
            assert_eq!(parts.join("/"), *path);
            path.clone()
        })
        .collect::<BTreeSet<_>>();
    let symlink_paths = case
        .symlinks
        .keys()
        .map(|path| {
            let parts = normal_parts(path);
            assert_eq!(parts.join("/"), *path);
            path.clone()
        })
        .collect::<BTreeSet<_>>();
    assert!(file_paths.is_disjoint(&symlink_paths));

    for (relative, contents) in &case.files {
        let path = owned_root.join(relative);
        assert!(path.starts_with(owned_root));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent");
        }
        fs::write(path, contents).expect("fixture file");
    }

    #[cfg(unix)]
    for (relative, target) in &case.symlinks {
        let virtual_target = resolve_virtual_link(relative, target);
        assert!(
            file_paths.contains(&virtual_target),
            "symlink target is not a declared file: {relative} -> {target}"
        );
        let link = owned_root.join(relative);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).expect("symlink parent");
        }
        std::os::unix::fs::symlink(target, &link).expect("fixture symlink");
        let canonical_root = fs::canonicalize(owned_root).expect("canonical fixture root");
        let canonical_target = fs::canonicalize(&link).expect("canonical symlink target");
        assert!(canonical_target.starts_with(&canonical_root));
    }
}

fn assert_case_shape(case: &Case) {
    assert_eq!(case.kind, "module_test_boundary");
    assert!(case.mode == "strict" || case.mode == "workspace");
    assert!(!case.crate_name.is_empty());
    let crate_root = normal_parts(&case.crate_root);
    let workspace_root = normal_parts(&case.workspace_root);
    let root_file = normal_parts(&case.root_file);
    assert!(!crate_root.is_empty());
    assert!(!workspace_root.is_empty());
    assert!(!root_file.is_empty());
    assert!(case.root_file.starts_with(&(case.crate_root.clone() + "/")));
    assert!(case.files.contains_key(&case.root_file));

    let file_paths = case
        .files
        .keys()
        .map(|path| {
            let parts = normal_parts(path);
            assert_eq!(parts.join("/"), *path);
            path.clone()
        })
        .collect::<BTreeSet<_>>();
    let link_paths = case
        .symlinks
        .keys()
        .map(|path| {
            let parts = normal_parts(path);
            assert_eq!(parts.join("/"), *path);
            path.clone()
        })
        .collect::<BTreeSet<_>>();
    assert!(file_paths.is_disjoint(&link_paths));
    for (path, target) in &case.symlinks {
        let resolved = resolve_virtual_link(path, target);
        assert!(file_paths.contains(&resolved));
    }

    let expected = &case.expected;
    assert!(expected.outcome == "accepted" || expected.outcome == "refused");
    let mut sorted_modules = expected.modules.clone();
    sorted_modules.sort();
    sorted_modules.dedup();
    assert_eq!(sorted_modules, expected.modules);
    if expected.outcome == "refused" {
        assert!(
            matches!(
                expected.reason_contains.as_deref(),
                Some("outside crate root")
                    | Some("outside permitted test source root")
                    | Some("crate root")
            ),
            "invalid refusal reason for {}",
            case.id
        );
        assert!(expected.modules.is_empty());
        assert!(expected.test_edge.is_none());
        assert!(expected.source_files.is_empty());
    } else {
        assert!(case.crate_root.starts_with(&(case.workspace_root.clone() + "/")));
        assert!(expected.reason_contains.is_none());
        assert!(!expected.modules.is_empty());
        assert!(expected.test_edge.is_some());
        assert!(!expected.source_files.is_empty());
    }
    for (path, digest) in &expected.source_files {
        assert!(file_paths.contains(path));
        assert!(!link_paths.contains(path));
        assert_eq!(digest.len(), 64);
        assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    if let Some(mutation) = &case.mutation {
        assert_eq!(expected.outcome, "accepted");
        assert!(file_paths.contains(&mutation.path));
        assert_eq!(mutation.expected_sha256.len(), 64);
        assert!(
            mutation
                .expected_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
    }
}

fn extract_case(case: &Case, owned_root: &Path) -> Result<Extracted, String> {
    let crate_root = owned_root.join(&case.crate_root);
    let root_file = owned_root.join(&case.root_file);
    let externals = BTreeSet::new();
    match case.mode.as_str() {
        "strict" => extract(&crate_root, &root_file, &case.crate_name, "2021", &externals),
        "workspace" => extract_with_test_root(
            &crate_root,
            &root_file,
            &owned_root.join(&case.workspace_root),
            &case.crate_name,
            "2021",
            &externals,
        ),
        mode => Err(format!("unsupported case mode: {mode}")),
    }
}

fn relative_source_map(owned_root: &Path, extracted: &Extracted) -> BTreeMap<String, String> {
    let canonical_root = fs::canonicalize(owned_root).expect("canonical fixture root");
    let mut actual = BTreeMap::new();
    for path in extracted.modules.values() {
        let canonical = fs::canonicalize(path).expect("canonical module source");
        assert!(canonical.starts_with(&canonical_root));
    }
    for (path, digest) in source_map_for_extracted(owned_root, extracted) {
        let path = PathBuf::from(path);
        assert!(path.is_absolute());
        assert_eq!(fs::canonicalize(&path).expect("canonical source map path"), path);
        let relative = path
            .strip_prefix(&canonical_root)
            .expect("source map path inside fixture root")
            .display()
            .to_string();
        actual.insert(relative, digest);
    }
    let module_paths = extracted
        .modules
        .values()
        .map(|path| {
            fs::canonicalize(path)
                .expect("canonical module source")
                .strip_prefix(&canonical_root)
                .expect("module source inside fixture root")
                .display()
                .to_string()
        })
        .collect::<BTreeSet<_>>();
    let source_paths = actual.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(module_paths, source_paths);
    actual
}

fn source_map_for_extracted(owned_root: &Path, extracted: &Extracted) -> BTreeMap<String, String> {
    let crate_root = owned_root
        .join("workspace/member")
        .canonicalize()
        .unwrap_or_else(|_| owned_root.join("workspace/member"));
    let workspace_root = owned_root.join("workspace");
    if workspace_root.exists() && crate_root.exists() {
        source_files_with_test_root(&crate_root, &workspace_root, extracted)
            .or_else(|_| source_files(&crate_root, extracted))
            .expect("source file digests")
    } else {
        source_files(&crate_root, extracted).expect("source file digests")
    }
}

#[test]
fn registered_module_test_boundary_cases_are_graded_exactly() {
    let package = package_root();
    let cases = verify_package(&package).expect("registered package");
    let ids = cases.iter().map(|case| case.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids.len(), CASE_IDS.len());
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>(),
        CASE_IDS.iter().copied().collect::<BTreeSet<_>>()
    );

    let copied = Scratch::new("copied-package");
    let copied_package = copied.path().join("package");
    copy_package(&package, &copied_package);
    verify_package(&copied_package).expect("verified copied positive package");
    let mut corrupted_cases =
        fs::read(&copied_package.join("CASES.json")).expect("copied CASES.json");
    corrupted_cases.push(b'\n');
    fs::write(copied_package.join("CASES.json"), corrupted_cases).expect("corrupt copied package");
    assert_eq!(
        verify_package(&copied_package).expect_err("corrupted package must fail closed"),
        "CASES.json digest is not the registered digest"
    );

    for case in cases {
        assert_case_shape(&case);
        let scratch = Scratch::new(&case.id);
        materialize_case(&case, scratch.path());
        let extracted = extract_case(&case, scratch.path());

        if case.expected.outcome == "refused" {
            let error = extracted.expect_err("refused case unexpectedly parsed");
            let reason = case.expected.reason_contains.as_deref().expect("refusal reason");
            assert!(
                error.contains(reason),
                "{}: expected {reason:?} in {error:?}",
                case.id
            );
            assert!(
                !error.contains("failed to parse"),
                "{}: boundary refusal must precede parsing: {error}",
                case.id
            );
            continue;
        }

        let extracted = extracted.unwrap_or_else(|error| panic!("{}: {error}", case.id));
        let actual_modules = extracted.modules.keys().cloned().collect::<Vec<_>>();
        assert_eq!(actual_modules, case.expected.modules, "{}", case.id);

        let expected_edge = case.expected.test_edge.as_ref().expect("test edge");
        assert_eq!(extracted.edges.len(), 1, "{}: edge count", case.id);
        let actual_edge = &extracted.edges[0];
        let module_endpoint = |endpoint: &str| {
            let mut candidate = endpoint;
            loop {
                if extracted.modules.contains_key(candidate) {
                    break candidate.to_owned();
                }
                candidate = candidate.rsplit_once("::").expect("internal module endpoint").0;
            }
        };
        assert_eq!(
            TestEdge {
                from: module_endpoint(&actual_edge.source),
                to: module_endpoint(&actual_edge.target),
            },
            *expected_edge,
            "{}: complete logical module edge",
            case.id
        );
        assert!(actual_edge.test_only);
        assert_eq!(actual_edge.extraction, "syntax");
        assert!(extracted.edges.iter().all(|edge| edge.test_only));

        let actual_sources = relative_source_map(scratch.path(), &extracted);
        assert_eq!(
            actual_sources, case.expected.source_files,
            "{}: complete source map",
            case.id
        );
        for (relative, expected_digest) in &case.expected.source_files {
            let path = scratch.path().join(relative);
            let bytes = fs::read(&path).expect("measured source bytes");
            assert_eq!(sha256(&bytes), *expected_digest, "{}: measured digest", case.id);
        }

        if let Some(mutation) = &case.mutation {
            fs::write(scratch.path().join(&mutation.path), &mutation.text)
                .expect("helper mutation");
            let mutated = extract_case(&case, scratch.path())
                .unwrap_or_else(|error| panic!("{} after mutation: {error}", case.id));
            assert_eq!(mutated.modules, extracted.modules, "{}: mutation modules", case.id);
            assert_eq!(mutated.edges, extracted.edges, "{}: mutation edges", case.id);
            let mutated_sources = relative_source_map(scratch.path(), &mutated);
            assert_eq!(mutated_sources.len(), actual_sources.len());
            for (path, digest) in &mutated_sources {
                if path == &mutation.path {
                    assert_eq!(digest, &mutation.expected_sha256, "{}: mutation digest", case.id);
                    assert_ne!(digest, actual_sources.get(path).expect("original digest"));
                } else {
                    assert_eq!(digest, actual_sources.get(path).expect("original digest"));
                }
            }
        }
    }
}

#[test]
fn an_inner_file_test_attribute_cannot_grant_external_read_authority() {
    let mut case = verify_package(&package_root())
        .expect("registered inputs")
        .into_iter()
        .find(|case| case.id == "nested_out_of_line_inherited_cfg_test_helper")
        .expect("nested input template");
    let root_source = case.files.get_mut(&case.root_file).expect("root source");
    assert_eq!(root_source.matches("#[cfg(test)]").count(), 1);
    *root_source = root_source.replace("#[cfg(test)]\n", "");
    let nested_path = format!("{}/src/checks.rs", case.crate_root);
    let nested = case.files.get_mut(&nested_path).expect("nested source");
    *nested = format!("#![cfg(test)]\n{nested}");
    let scratch = Scratch::new("inner-file-authority");
    materialize_case(&case, scratch.path());
    let error = extract_case(&case, scratch.path())
        .expect_err("an inner file attribute is not declaration ancestry");
    assert!(error.contains("outside crate root"), "{error}");
}

#[test]
fn a_root_inner_test_attribute_cannot_grant_external_read_authority() {
    let mut case = verify_package(&package_root())
        .expect("registered inputs")
        .into_iter()
        .find(|case| case.id == "workspace_external_cfg_test_helper")
        .expect("direct input template");
    let source = case.files.get_mut(&case.root_file).expect("root source");
    assert_eq!(source.matches("#[cfg(test)]").count(), 1);
    *source = format!("#![cfg(test)]\n{}", source.replace("#[cfg(test)]\n", ""));
    let scratch = Scratch::new("root-inner-authority");
    materialize_case(&case, scratch.path());
    let error = extract_case(&case, scratch.path())
        .expect_err("a root inner attribute is not declaration ancestry");
    assert!(error.contains("outside crate root"), "{error}");
}

#[test]
fn inner_test_attributes_still_classify_in_crate_edges_as_test_only() {
    let scratch = Scratch::new("inner-classification");
    let crate_root = scratch.path().join("member");
    let source_root = crate_root.join("src");
    fs::create_dir_all(&source_root).expect("source directory");
    let root_file = source_root.join("lib.rs");
    fs::write(&root_file, "#![cfg(test)]\npub fn root_only(){}\nmod checks;\n")
        .expect("root source");
    fs::write(source_root.join("checks.rs"), "pub fn check(){crate::root_only();}\n")
        .expect("child source");
    let extracted = extract(&crate_root, &root_file, "x", "2021", &BTreeSet::new())
        .expect("strict in-crate extraction");
    assert_eq!(extracted.edges.len(), 1);
    assert!(extracted.edges[0].test_only);
}
