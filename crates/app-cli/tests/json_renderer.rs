#[path = "../../adapter-json/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

fn temp_root() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "rha-json-cli-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    root
}

struct OwnedTempDir(PathBuf);

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_sources(root: &Path, sources: &Value) {
    for source in sources.as_array().unwrap() {
        let path = source["path"].as_str().unwrap();
        let rel = library::RelPath::new(path).unwrap();
        let target = root.join(rel.as_str());
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(target, source["text"].as_str().unwrap()).unwrap();
    }
}

fn files(root: &Path, relative: &Path, output: &mut Vec<String>) {
    let current = root.join(relative);
    let metadata = fs::symlink_metadata(&current).unwrap();
    assert!(!metadata.file_type().is_symlink());
    if metadata.is_dir() {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            files(root, &relative.join(entry.file_name()), output);
        }
    } else {
        output.push(
            relative
                .to_str()
                .unwrap()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        );
    }
}

fn output_paths(root: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    files(root, Path::new(""), &mut paths);
    paths.sort();
    paths
}

fn run_build(source: &Path, output: &Path, format: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rhawiki"));
    command
        .arg("build")
        .arg("--root")
        .arg(source)
        .arg("--out")
        .arg(output);
    if let Some(format) = format {
        command.arg("--format").arg(format);
    }
    command.output().expect("rhawiki build must start")
}

#[test]
fn registered_cli_cases_are_exactly_graded() {
    let cases = common::cases();
    let mut completed = 0;
    for case in &cases {
        match case["kind"].as_str().unwrap() {
            "cli_json" => assert_cli_json(case),
            "cli_html" => assert_cli_html(case),
            "cli_check" => assert_cli_check(case),
            "collision" | "contract" | "duplicate" | "project" | "reconcile" => continue,
            other => panic!("unknown registered case kind {other}"),
        }
        completed += 1;
    }
    assert_eq!(completed, 4);
    println!("CLI registered cases completed: {completed}");
}

fn assert_cli_json(case: &Value) {
    let root = temp_root();
    let _guard = OwnedTempDir(root.clone());
    let source = root.join("source");
    let output = root.join("output");
    fs::create_dir(&source).unwrap();
    write_sources(&source, &case["sources"]);
    let result = run_build(&source, &output, Some("json"));
    assert_eq!(
        result.status.code(),
        Some(case["expected_exit"].as_i64().unwrap() as i32)
    );
    assert_eq!(
        serde_json::json!(output_paths(&output)),
        case["expected_paths"]
    );

    let expected_titles = case["expected_titles"].as_object().unwrap();
    for (id, title) in expected_titles {
        let path = output.join(format!("{id}.json"));
        let page: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(
            page["schema_version"],
            case["expected_page_schema"]["schema_version"]
        );
        assert_eq!(page["kind"], case["expected_page_schema"]["kind"]);
        for member in case["required_page_members"].as_array().unwrap() {
            assert!(
                page.get(member.as_str().unwrap()).is_some(),
                "missing {member}"
            );
        }
        assert_eq!(&page["title"], title);
        assert_eq!(&page["toc"], &case["expected_toc"][id]);
    }

    let index: Value =
        serde_json::from_slice(&fs::read(output.join("assets/search-index.json")).unwrap())
            .unwrap();
    let ids = index["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(serde_json::json!(ids), case["expected_index_ids"]);
}

fn assert_cli_html(case: &Value) {
    let root = temp_root();
    let _guard = OwnedTempDir(root.clone());
    let source = root.join("source");
    let output = root.join("output");
    fs::create_dir(&source).unwrap();
    write_sources(&source, &case["sources"]);
    let format = case["explicit_format"].as_bool().unwrap().then_some("html");
    let result = run_build(&source, &output, format);
    assert_eq!(
        result.status.code(),
        Some(case["expected_exit"].as_i64().unwrap() as i32)
    );
    assert_eq!(
        serde_json::json!(output_paths(&output)),
        case["expected_paths"]
    );
    for (path, markers) in case["expected_markers"].as_object().unwrap() {
        let text = fs::read_to_string(output.join(path)).unwrap();
        for marker in markers.as_array().unwrap() {
            assert!(
                text.contains(marker.as_str().unwrap()),
                "missing marker {marker}"
            );
        }
    }
}

fn assert_cli_check(case: &Value) {
    let root = temp_root();
    let _guard = OwnedTempDir(root.clone());
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    write_sources(&source, &case["sources"]);
    let result = Command::new(env!("CARGO_BIN_EXE_rhawiki"))
        .arg("check")
        .arg("--root")
        .arg(&source)
        .arg("--format")
        .arg("json")
        .output()
        .expect("rhawiki check must start");
    assert_eq!(
        result.status.code(),
        Some(case["expected_exit"].as_i64().unwrap() as i32)
    );
    assert!(result.stdout.ends_with(b"\n"));
    let actual: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(actual, case["expected_json"]);
}
