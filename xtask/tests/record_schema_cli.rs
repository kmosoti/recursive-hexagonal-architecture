use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a repository parent")
        .to_path_buf()
}

fn corpus_root() -> PathBuf {
    repository_root().join("xtask/tests/corpus/record-schema")
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rha-record-schema-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temporary directory");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn decode_pointer(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    assert!(path.starts_with('/'), "invalid JSON Pointer {path:?}");
    path[1..]
        .split('/')
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn index(segment: &str, path: &str) -> usize {
    segment
        .parse()
        .unwrap_or_else(|_| panic!("invalid array index {segment:?} in {path}"))
}

fn child_mut<'a>(value: &'a mut Value, segment: &str, path: &str) -> &'a mut Value {
    match value {
        Value::Object(object) => object
            .get_mut(segment)
            .unwrap_or_else(|| panic!("missing JSON Pointer parent {path}")),
        Value::Array(array) => array
            .get_mut(index(segment, path))
            .unwrap_or_else(|| panic!("missing JSON Pointer parent {path}")),
        _ => panic!("non-container JSON Pointer parent for {path}"),
    }
}

fn apply_mutation(value: &mut Value, mutation: &Value) {
    let op = mutation["op"].as_str().expect("mutation operation");
    let path = mutation["path"].as_str().expect("mutation path");
    let parts = decode_pointer(path);
    if parts.is_empty() {
        match op {
            "set" | "replace" => *value = mutation["value"].clone(),
            "remove" => panic!("root removal is not a defined recipe"),
            _ => panic!("unknown mutation operation {op}"),
        }
        return;
    }

    let last = parts.last().expect("non-empty pointer").clone();
    let parent = parts[..parts.len() - 1]
        .iter()
        .fold(value, |current, segment| child_mut(current, segment, path));
    match parent {
        Value::Object(object) => match op {
            "set" => {
                object.insert(last, mutation["value"].clone());
            }
            "replace" => {
                assert!(
                    object.contains_key(&last),
                    "replace target is absent: {path}"
                );
                object.insert(last, mutation["value"].clone());
            }
            "remove" => {
                assert!(
                    object.remove(&last).is_some(),
                    "remove target is absent: {path}"
                );
            }
            _ => panic!("unknown mutation operation {op}"),
        },
        Value::Array(array) => {
            let position = index(&last, path);
            match op {
                "set" | "replace" => {
                    assert!(position < array.len(), "array target is absent: {path}");
                    array[position] = mutation["value"].clone();
                }
                "remove" => {
                    assert!(position < array.len(), "array target is absent: {path}");
                    array.remove(position);
                }
                _ => panic!("unknown mutation operation {op}"),
            }
        }
        _ => panic!("non-container JSON Pointer parent for {path}"),
    }
}

fn parse_source(path: &Path) -> Value {
    let bytes = std::fs::read(path).expect("recipe source");
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        serde_json::from_slice(&bytes).expect("JSON recipe source")
    } else {
        let toml_value: toml::Value =
            toml::from_str(std::str::from_utf8(&bytes).expect("UTF-8 TOML source"))
                .expect("TOML recipe source");
        serde_json::to_value(toml_value).expect("TOML converts to JSON")
    }
}

fn recipe_value(id: &str) -> Value {
    let cases: Value = serde_json::from_slice(
        &std::fs::read(corpus_root().join("CASES.json")).expect("record-schema cases"),
    )
    .expect("record-schema cases JSON");
    let case = cases["cases"]
        .as_array()
        .expect("cases array")
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("missing recipe {id}"));
    let baseline_name = case["baseline"].as_str().expect("baseline name");
    let baseline = &cases["baselines"][baseline_name];
    let source = baseline["source"].as_str().expect("baseline source");
    let source_path = corpus_root().join(source);
    let bytes = std::fs::read(&source_path).expect("baseline source bytes");
    let expected_hash = baseline["source_sha256"]
        .as_str()
        .expect("baseline source hash");
    assert_eq!(
        xtask::util::sha256_hex(&bytes),
        expected_hash,
        "source hash changed: {source}"
    );

    let mut value = parse_source(&source_path);
    for mutation in baseline["mutations"]
        .as_array()
        .expect("baseline mutations")
    {
        apply_mutation(&mut value, mutation);
    }
    for mutation in case["mutations"].as_array().expect("case mutations") {
        apply_mutation(&mut value, mutation);
    }
    value
}

fn write_json(path: &Path, value: &Value) {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize JSON record"),
    )
    .expect("write JSON record");
}

fn run_lint(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["rha", "lint"])
        .arg(path)
        .current_dir(repository_root())
        .output()
        .expect("spawn xtask")
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn cli_schema_controls_are_rejected_or_accepted_as_registered() {
    let temp = TempDir::new("schema-controls");
    let path = temp.path.join("record.json");

    write_json(&path, &recipe_value("RS-cli-safe-l0-control"));
    let output = run_lint(&path);
    assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));

    write_json(&path, &recipe_value("RS-cli-missing-outcome"));
    let output = run_lint(&path);
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    assert!(output_text(&output).contains("schema.missing_field"));

    write_json(&path, &recipe_value("RS-cli-unknown-outcome"));
    let output = run_lint(&path);
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    assert!(output_text(&output).contains("schema.invalid_value"));

    write_json(&path, &recipe_value("RS-cli-unsupported-report-kind"));
    let output = run_lint(&path);
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    assert!(output_text(&output).contains("schema.invalid_value"));
}

#[test]
fn cli_classifies_report_inputs_and_commented_tasks_correctly() {
    let temp = TempDir::new("classification");
    let task_path = temp.path.join("task.toml");
    let task_source = repository_root()
        .join("xtask/tests/corpus/record-schema/sources/.rha/tasks/CHG-019-trust.toml");
    let mut task = std::fs::read_to_string(task_source).expect("task source");
    task.push_str("\n# [acceptor]\n");
    std::fs::write(&task_path, task).expect("commented task");
    let output = run_lint(&task_path);
    assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));
    assert!(
        output_text(&output).contains("(task)"),
        "{}",
        output_text(&output)
    );

    let source_index: Value = serde_json::from_slice(
        &std::fs::read(corpus_root().join("SOURCE-INDEX.json")).expect("source index"),
    )
    .expect("source index JSON");
    for family in ["markdown_corpus", "h5_conformance", "h4"] {
        let source = source_index["sources"]
            .as_array()
            .expect("sources")
            .iter()
            .find(|source| source["family"] == family)
            .expect("registered family");
        let path = corpus_root().join(source["snapshot"].as_str().expect("snapshot"));
        let output = run_lint(&path);
        assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));
        assert!(
            !output_text(&output).contains("evidence.missing_required_check"),
            "{}",
            output_text(&output)
        );
    }
}

#[test]
fn cli_reports_unchecked_and_checked_citations_without_following_escape_symlinks() {
    let temp = TempDir::new("citations");
    let record_dir = temp.path.join("records");
    std::fs::create_dir_all(&record_dir).expect("record directory");
    let path = record_dir.join("record.json");

    let mut value = recipe_value("RS-cli-safe-l0-control");
    value["verification_identity"]["instruction_sources"] = serde_json::json!([{
        "path": "beside.txt",
        "sha256": format!("sha256:{}", "0".repeat(64))
    }]);
    std::fs::write(record_dir.join("beside.txt"), "known content\n").expect("beside file");
    let output = {
        write_json(&path, &value);
        run_lint(&path)
    };
    assert_eq!(output.status.code(), Some(1), "{}", output_text(&output));
    assert!(output_text(&output).contains("digest.mismatch"));

    let digest = xtask::util::sha256_hex(b"known content\n");
    value["verification_identity"]["instruction_sources"] = serde_json::json!([{
        "path": "beside.txt",
        "sha256": format!("sha256:{digest}")
    }]);
    let output = {
        write_json(&path, &value);
        run_lint(&path)
    };
    assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));
    assert!(!output_text(&output).contains("digest.mismatch"));

    let missing = format!(
        ".rha-record-schema-missing-{}-{}.txt",
        std::process::id(),
        digest
    );
    value["verification_identity"]["instruction_sources"] = serde_json::json!([{
        "path": missing,
        "sha256": format!("sha256:{}", "0".repeat(64))
    }]);
    let output = {
        write_json(&path, &value);
        run_lint(&path)
    };
    assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));
    let text = output_text(&output);
    assert!(text.contains("citation.unchecked"), "{text}");
    assert!(text.contains("accepted with unchecked citations"), "{text}");
    assert!(!text.contains(": accepted ("), "{text}");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside = temp.path.join("outside.txt");
        std::fs::write(&outside, "outside content\n").expect("outside file");
        symlink(&outside, record_dir.join("escape.txt")).expect("escape symlink");
        let outside_digest = xtask::util::sha256_hex(b"outside content\n");
        value["verification_identity"]["instruction_sources"] = serde_json::json!([{
            "path": "escape.txt",
            "sha256": format!("sha256:{outside_digest}")
        }]);
        let output = {
            write_json(&path, &value);
            run_lint(&path)
        };
        assert_eq!(output.status.code(), Some(0), "{}", output_text(&output));
        let text = output_text(&output);
        assert!(text.contains("citation.unchecked"), "{text}");
        assert!(text.contains("accepted with unchecked citations"), "{text}");
        assert!(!text.contains("digest.mismatch"), "{text}");
    }
}
