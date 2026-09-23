use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use xtask::corpus::module::{validate_report, verify_inventory, verify_registration_contract};

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(root: &Path) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let parent = root.join("target/rha");
        fs::create_dir_all(&parent).expect("scratch parent");
        for _ in 0..256 {
            let counter = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("checker-controls-{}-{counter}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("creating {}: {error}", path.display()),
            }
        }
        panic!("could not allocate a checker-controls scratch directory");
    }

    fn module_root(&self) -> PathBuf {
        self.path.join("xtask/tests/corpus/module")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path)
            && self.path.exists()
        {
            eprintln!("failed to remove {}: {error}", self.path.display());
        }
    }
}

fn root() -> PathBuf {
    fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root"),
    )
    .expect("canonical workspace root")
}

fn copy_tree(source: &Path, destination: &Path) {
    let metadata = fs::symlink_metadata(source).expect("source metadata");
    assert!(
        !metadata.file_type().is_symlink(),
        "frozen source must not contain symlinks: {}",
        source.display()
    );

    if metadata.is_dir() {
        assert_ne!(
            source.file_name().and_then(|name| name.to_str()),
            Some("target"),
            "generated target directories are not corpus inputs"
        );
        fs::create_dir_all(destination).expect("destination directory");
        let mut entries = fs::read_dir(source)
            .expect("source directory")
            .map(|entry| entry.expect("source entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            copy_tree(&entry.path(), &destination.join(entry.file_name()));
        }
    } else {
        assert!(metadata.is_file(), "source must be a durable file");
        fs::create_dir_all(destination.parent().expect("destination parent"))
            .expect("destination parent");
        fs::copy(source, destination).expect("copy frozen file");
    }
}

fn frozen_copy(root: &Path) -> Scratch {
    let scratch = Scratch::new(root);
    let source = root.join("xtask/tests/corpus");
    let destination = scratch.path.join("xtask/tests/corpus");
    copy_tree(&source.join("module"), &destination.join("module"));
    fs::create_dir_all(&destination).expect("corpus parent");
    fs::copy(
        source.join("manifest.toml"),
        destination.join("manifest.toml"),
    )
    .expect("frozen manifest");
    scratch
}

fn assert_frozen_control(module_root: &Path) {
    let payloads = verify_inventory(module_root).expect("frozen inventory");
    verify_registration_contract(module_root, &payloads).expect("frozen registration contract");
}

#[test]
fn copied_module_inventory_controls_reject_frozen_input_mutations() {
    let root = root();
    let scratch = frozen_copy(&root);
    let module_root = scratch.module_root();

    assert_frozen_control(&module_root);

    let source_path = module_root.join("random/R0000.json");
    let expected_path = module_root.join("random/R0000.expected.json");
    let source_bytes = fs::read(&source_path).expect("source map");
    let expected_bytes = fs::read(&expected_path).expect("expected output");
    let mut corrupted_source = source_bytes.clone();
    let mut corrupted_expected = expected_bytes.clone();
    corrupted_source.extend_from_slice(b"\ncontrol-corruption");
    corrupted_expected.extend_from_slice(b"\ncontrol-corruption");
    fs::write(&source_path, corrupted_source).expect("corrupt source map");
    fs::write(&expected_path, corrupted_expected).expect("corrupt expected output");
    assert!(
        verify_inventory(&module_root).is_err(),
        "paired source and expected corruption must fail inventory verification"
    );
    fs::write(&source_path, source_bytes).expect("restore source map");
    fs::write(&expected_path, expected_bytes).expect("restore expected output");
    assert_frozen_control(&module_root);

    let rule_expectations = module_root.join("random-rule-expectations.json");
    let rule_expectation_bytes = fs::read(&rule_expectations).expect("rule expectations");
    fs::remove_file(&rule_expectations).expect("remove rule expectations");
    assert!(
        xtask::corpus::module_random::run(&scratch.path).is_err(),
        "missing required random rule expectations must fail"
    );
    fs::write(&rule_expectations, rule_expectation_bytes).expect("restore rule expectations");
    assert_frozen_control(&module_root);

    let extra = module_root.join("random/extra.rs");
    fs::write(&extra, b"unexpected corpus input\n").expect("extra corpus input");
    assert!(
        verify_inventory(&module_root).is_err(),
        "an unregistered corpus file must fail inventory verification"
    );
    fs::remove_file(&extra).expect("remove extra corpus input");
    assert_frozen_control(&module_root);

    let sums_path = module_root.join("SHA256SUMS");
    let sums_bytes = fs::read(&sums_path).expect("SHA256SUMS");
    let mut tampered_sums = sums_bytes.clone();
    let replacement = if tampered_sums[0] == b'0' { b'1' } else { b'0' };
    tampered_sums[..64].fill(replacement);
    fs::write(&sums_path, tampered_sums).expect("tamper SHA256SUMS");
    assert!(
        verify_inventory(&module_root).is_err(),
        "a syntactically valid but tampered SHA256SUMS must fail its fixed commitment"
    );
    fs::write(&sums_path, sums_bytes).expect("restore SHA256SUMS");
    assert_frozen_control(&module_root);
}

fn assert_report_rejected(root: &Path, fixture: &Path, report: &Value, tool: &Value) {
    assert!(
        validate_report(root, fixture, report, tool).is_err(),
        "tampered report was accepted: {report}"
    );
}

#[test]
fn module_report_validation_binds_the_real_invocation() {
    let root = root();
    let fixture = root.join("xtask/tests/corpus/module/headline/M01");
    let manifest = fixture.join("Cargo.toml");
    let rules = fixture.join("rha-modules.toml");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["architecture", "--manifest-path"])
        .arg(&manifest)
        .args(["--module-rules"])
        .arg(&rules)
        .args(["--format", "json"])
        .current_dir(&root)
        .output()
        .expect("run module-only checker");

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "module-only checker did not emit JSON: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });

    let tool = xtask::graph::report::tool_identity(&root);
    assert_eq!(report["tool"], tool);
    validate_report(&root, &fixture, &report, &tool)
        .expect("real M01 module-only report should validate");

    let mut producer = report.clone();
    producer["tool"]["name"] = json!("tampered producer");
    assert_report_rejected(&root, &fixture, &producer, &tool);

    let mut rules_digest = report.clone();
    rules_digest["subject"]["rules_digest"] =
        json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    assert_report_rejected(&root, &fixture, &rules_digest, &tool);

    let mut not_run = report.clone();
    not_run["module_checks"][0]["outcome"] = json!("not_run");
    assert_report_rejected(&root, &fixture, &not_run, &tool);

    let mut source_inputs_absent = report.clone();
    source_inputs_absent["subject"]
        .as_object_mut()
        .expect("subject object")
        .remove("source_files_sha256");
    for check in source_inputs_absent["module_checks"]
        .as_array_mut()
        .expect("module checks array")
    {
        check
            .as_object_mut()
            .expect("module check object")
            .remove("source_files_sha256");
    }
    assert_report_rejected(&root, &fixture, &source_inputs_absent, &tool);
}
