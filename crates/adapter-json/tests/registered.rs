#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use adapter_json::{JsonRenderer, JsonRendererError};
use common::cases;
use library::{Digest, RelPath};
use serde_json::Value;
use site::SinkError;
use site::testing::{FixedClock, RecordingSink};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct OwnedTempDir(PathBuf);

struct CountingSink {
    inner: RecordingSink,
    write_calls: usize,
    delete_calls: usize,
}

impl site::OutputSink for CountingSink {
    fn list(&self) -> Result<Vec<(RelPath, Digest)>, SinkError> {
        site::OutputSink::list(&self.inner)
    }

    fn write(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), SinkError> {
        self.write_calls += 1;
        site::OutputSink::write(&mut self.inner, path, bytes)
    }

    fn delete(&mut self, path: &RelPath) -> Result<(), SinkError> {
        self.delete_calls += 1;
        site::OutputSink::delete(&mut self.inner, path)
    }
}

impl Drop for OwnedTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_root(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    root
}

#[test]
fn registered_json_adapter_cases_are_all_graded() {
    let cases = cases();
    let mut completed = 0;
    let mut counts = BTreeMap::<&str, usize>::new();
    for case in &cases {
        let kind = case["kind"].as_str().unwrap();
        match kind {
            "project" => common::assert_project_case(case),
            "duplicate" => assert_duplicate_case(case),
            "reconcile" => assert_reconcile_case(case),
            "collision" => assert_collision_case(case),
            "contract" => assert_contract_case(case),
            "cli_check" | "cli_html" | "cli_json" => continue,
            other => panic!("unknown registered case kind {other}"),
        }
        completed += 1;
        *counts.entry(kind).or_default() += 1;
    }
    assert_eq!(completed, 75);
    assert_eq!(
        counts,
        BTreeMap::from([
            ("collision", 1),
            ("contract", 1),
            ("duplicate", 2),
            ("project", 70),
            ("reconcile", 1),
        ])
    );
    println!("adapter-json registered cases completed: {completed}");
}

fn assert_duplicate_case(case: &Value) {
    let pages = common::models_from_values(&case["pages"]);
    let error = JsonRenderer::new(&pages).expect_err("duplicate IDs must be rejected");
    match error {
        JsonRendererError::DuplicatePageId { id } => {
            assert_eq!(id, case["expected_error"]["id"].as_str().unwrap());
            assert_eq!(case["expected_error"]["kind"], "DuplicatePageId");
        }
    }
}

fn assert_reconcile_case(case: &Value) {
    assert_eq!(case["clock_type"], "FixedClock");
    assert!(case["fresh_renderer_each_build"].as_bool().unwrap());
    let clock = case["clock"].as_str().unwrap();
    let before = common::corpus_from_sources(&case["before_sources"]);
    let after = common::corpus_from_sources(&case["after_sources"]);
    let mut sink = RecordingSink::default();

    let before_models = common::models_from_corpus(&before, clock);
    let before_renderer = JsonRenderer::new(&before_models).unwrap();
    let first = site::build_all(
        &before,
        &FixedClock(clock.to_owned()),
        &mut sink,
        &before_renderer,
    )
    .expect("initial build must succeed");
    assert_eq!(
        serde_json::json!(common::sink_paths(&sink)),
        case["expected_initial_paths"]
    );
    assert_eq!(first.unchanged, 0);

    let after_models = common::models_from_corpus(&after, clock);
    let after_renderer = JsonRenderer::new(&after_models).unwrap();
    let second = site::build_all(
        &after,
        &FixedClock(clock.to_owned()),
        &mut sink,
        &after_renderer,
    )
    .expect("reconciled build must succeed");
    assert_eq!(
        serde_json::json!(common::sink_paths(&sink)),
        case["expected_final_paths"]
    );
    assert_eq!(
        serde_json::json!(
            second
                .deleted
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>()
        ),
        case["expected_deleted_paths"]
    );
    assert_eq!(
        second.unchanged,
        case["expected_unchanged"].as_u64().unwrap() as usize
    );
    let index = sink
        .files
        .get(&library::RelPath::new("assets/search-index.json").unwrap())
        .unwrap();
    assert_eq!(
        serde_json::json!(common::index_ids(index)),
        case["expected_index_ids"]
    );
}

fn assert_collision_case(case: &Value) {
    let corpus = common::corpus_from_sources(&case["sources"]);
    let models = common::models_from_corpus(&corpus, "t0");
    let renderer = JsonRenderer::new(&models).unwrap();
    let mut sink = CountingSink {
        inner: RecordingSink::default(),
        write_calls: 0,
        delete_calls: 0,
    };
    let error = site::build_all(&corpus, &FixedClock("t0".to_owned()), &mut sink, &renderer)
        .expect_err("page/asset collision must fail");
    match error {
        site::SiteError::InvariantViolation(message) => {
            assert!(message.contains(case["expected_error_contains"].as_str().unwrap()));
        }
        other => panic!("unexpected collision error {other:?}"),
    }
    assert_eq!(
        sink.inner.files.len(),
        case["expected_sink_files"].as_u64().unwrap() as usize
    );
    assert_eq!(sink.write_calls, 0);
    assert_eq!(sink.delete_calls, 0);
}

fn assert_contract_case(case: &Value) {
    assert!(case["owner_contract_passed"].as_bool().unwrap());
    let models = common::models_from_values(&case["pages"]);
    let renderer = JsonRenderer::new(&models).unwrap();
    let violations = site::contract::page_renderer(&renderer, &models);
    assert_eq!(violations, Vec::<site::contract::RendererViolation>::new());
    assert_eq!(case["expected_violations"], serde_json::json!([]));
}

#[test]
fn corrupted_cases_copy_is_refused_without_touching_fixture() {
    let root = temp_root("rha-json-corrupt");
    let _guard = OwnedTempDir(root.clone());
    let mut bytes = fs::read(common::corpus_root().join("CASES.json")).unwrap();
    bytes[0] = b' ';
    fs::write(root.join("CASES.json"), bytes).unwrap();
    let error = common::load_cases(&root).expect_err("corrupted CASES must be refused");
    assert!(error.contains("CASES.json"));
}

#[cfg(unix)]
#[test]
fn symlinked_cases_file_is_refused_before_reading() {
    use std::os::unix::fs::symlink;

    let root = temp_root("rha-json-symlink");
    let _guard = OwnedTempDir(root.clone());
    let harmless = root.join("harmless.txt");
    fs::write(&harmless, b"harmless").unwrap();
    symlink(&harmless, root.join("CASES.json")).unwrap();

    let error = common::verify_fixture(&root).expect_err("symlinked CASES must be refused");
    assert!(error.contains("symlink"), "{error}");
}
