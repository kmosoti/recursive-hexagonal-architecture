//! `cargo xtask corpus held-out` on public controls. The runner is exercised
//! on committed fixtures only; the private cases are never in a test
//! (DP-1.1b).

use std::path::{Path, PathBuf};
use std::process::Command;

use xtask::cli::HeldOutArgs;
use xtask::corpus::held_out;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn temp(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rha-held-out-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy target");
    for entry in std::fs::read_dir(from).expect("read fixture") {
        let entry = entry.expect("entry");
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let dest = to.join(&name);
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy file");
        }
    }
}

#[test]
fn members_that_would_escape_are_rejected() {
    assert!(!held_out::safe_member("../evil"));
    assert!(!held_out::safe_member("/etc/passwd"));
    assert!(!held_out::safe_member("ok/../../evil"));
    assert!(held_out::safe_member("case/Cargo.toml"));
    assert!(held_out::safe_member("./case"));
}

#[test]
fn public_controls_are_observed_without_naming_them() {
    let root = root();
    let cases = temp("cases");
    copy_dir(
        &root.join("xtask/tests/corpus/crate/legitimate/L01"),
        &cases.join("one"),
    );
    copy_dir(
        &root.join("xtask/tests/corpus/crate/violations/C01"),
        &cases.join("two"),
    );
    let private = temp("private");
    let args = HeldOutArgs {
        cases: Some(cases.clone()),
        archive: None,
        purpose: "public_control".to_owned(),
        private_dir: Some(private.clone()),
        kind: "architecture".to_owned(),
    };
    let (summary, code) = held_out::execute(&root, &args).expect("runner");
    assert_eq!(code, 0, "{summary}");
    assert_eq!(summary["outcome"], "observed");
    assert_eq!(summary["graded"], false);
    let rows = summary["cases"].as_array().expect("cases");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["case_id"], "H001");
    assert_eq!(rows[0]["exit_status"], 0);
    assert_eq!(rows[0]["errors"], 0);
    assert_eq!(rows[1]["case_id"], "H002");
    assert_eq!(rows[1]["exit_status"], 1);
    assert_eq!(rows[1]["errors"], 1);
    assert!(
        matches!(
            summary["commitment"]["status"].as_str(),
            Some("mismatched" | "unverified")
        ),
        "public controls are not the committed cases: {}",
        summary["commitment"]
    );
    let text = serde_json::to_string(&summary).expect("json");
    let location = cases.to_str().expect("utf8");
    assert!(
        !text.contains(location),
        "the summary must not name the case location"
    );
    assert!(!text.contains("L01") && !text.contains("C01") && !text.contains("core-a"));
    let reports = PathBuf::from(summary["private_reports"].as_str().expect("private dir"));
    assert!(reports.starts_with(&private));
    for name in [
        "summary.json",
        "private-case-map.json",
        "H001.stdout",
        "H002.stdout",
    ] {
        assert!(reports.join(name).is_file(), "{name}");
    }
    let map = std::fs::read_to_string(reports.join("private-case-map.json")).expect("map");
    assert!(map.contains("\"H002\": \"two\""), "{map}");
    std::fs::remove_dir_all(cases).expect("clean cases");
    std::fs::remove_dir_all(private).expect("clean private");
}

#[test]
fn a_directory_is_verified_by_the_committed_stream_and_an_archive_by_its_bytes() {
    let dir = temp("commit");
    std::fs::write(dir.join("a.txt"), "held out\n").expect("file");
    let stream = Command::new("tar")
        .args(held_out::COMMITMENT_TAR_FLAGS)
        .args(["-cf", "-", "-C"])
        .arg(&dir)
        .arg(".")
        .output();
    let Ok(stream) = stream else {
        eprintln!("tar unavailable; the runner reports unverified");
        assert_eq!(
            held_out::verify_directory(&dir, "sha256:0")["status"],
            "unverified"
        );
        return;
    };
    assert!(stream.status.success());
    let expected = format!("sha256:{}", xtask::util::sha256_hex(&stream.stdout));
    assert_eq!(
        held_out::verify_directory(&dir, &expected)["status"],
        "matched"
    );
    assert_eq!(
        held_out::verify_directory(&dir, "sha256:0")["status"],
        "mismatched"
    );
    let archive = temp("archive").join("cases.tar");
    std::fs::write(&archive, &stream.stdout).expect("archive");
    assert_eq!(
        held_out::verify_archive(&archive, &expected).expect("hash")["status"],
        "matched"
    );
    let dest = temp("extract");
    held_out::extract(&archive, &dest).expect("extract");
    assert_eq!(
        std::fs::read_to_string(dest.join("a.txt")).expect("extracted"),
        "held out\n"
    );
    std::fs::remove_dir_all(dir).expect("clean");
    std::fs::remove_dir_all(dest).expect("clean");
}

#[test]
fn a_mismatched_archive_is_not_run_for_the_held_out_purpose() {
    let archive = temp("mismatch").join("not-the-cases.tar");
    std::fs::write(&archive, b"not a tar").expect("file");
    let private = temp("private-mismatch");
    let args = HeldOutArgs {
        cases: None,
        archive: Some(archive.clone()),
        purpose: "held_out".to_owned(),
        private_dir: Some(private.clone()),
        kind: "architecture".to_owned(),
    };
    let (summary, code) = held_out::execute(&root(), &args).expect("runner");
    assert_eq!(code, 2);
    assert_eq!(summary["outcome"], "not_run");
    assert_eq!(summary["commitment"]["status"], "mismatched");
    assert!(
        !private.join("inputs").exists(),
        "nothing is extracted before the commitment matches"
    );
    std::fs::remove_dir_all(private).expect("clean");
}

#[test]
fn a_directory_that_does_not_verify_is_not_run_for_the_held_out_purpose() {
    let root = root();
    let cases = temp("unverified");
    copy_dir(
        &root.join("xtask/tests/corpus/crate/legitimate/L01"),
        &cases.join("one"),
    );
    let private = temp("private-unverified");
    let args = HeldOutArgs {
        cases: Some(cases.clone()),
        archive: None,
        purpose: "held_out".to_owned(),
        private_dir: Some(private.clone()),
        kind: "architecture".to_owned(),
    };
    let (summary, code) = held_out::execute(&root, &args).expect("runner");
    assert_eq!(code, 2, "{summary}");
    assert_eq!(summary["outcome"], "not_run");
    assert!(summary.get("cases").is_none(), "nothing was observed");
    std::fs::remove_dir_all(cases).expect("clean");
    std::fs::remove_dir_all(private).expect("clean");
}

#[test]
fn link_members_are_refused_before_extraction() {
    assert!(held_out::only_files_and_directories(
        "drwxr-xr-x 0/0 0 2026-09-20 00:00 ./\n-rw-r--r-- 0/0 2 2026-09-20 00:00 ./a\n"
    ));
    assert!(!held_out::only_files_and_directories(
        "lrwxrwxrwx 0/0 0 2026-09-20 00:00 ./link -> /etc\n"
    ));
    assert!(!held_out::only_files_and_directories(
        "hrw-r--r-- 0/0 0 2026-09-20 00:00 ./b link to ./a\n"
    ));
    let dir = temp("links");
    std::fs::write(dir.join("a"), "x").expect("file");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/etc", dir.join("link")).expect("symlink");
    let archive = temp("links-archive").join("t.tar");
    let made = Command::new("tar")
        .arg("-cf")
        .arg(&archive)
        .arg("-C")
        .arg(&dir)
        .arg(".")
        .status();
    if made.is_ok_and(|s| s.success()) {
        let dest = temp("links-dest");
        assert!(held_out::extract(&archive, &dest).is_err());
        assert!(!dest.join("a").exists(), "nothing is extracted");
        std::fs::remove_dir_all(dest).expect("clean");
    }
    std::fs::remove_dir_all(dir).expect("clean");
}

#[test]
fn markdown_sites_are_observed_as_counts_only() {
    let root = root();
    let cases = temp("md");
    copy_dir(
        &root.join("xtask/tests/corpus/markdown/sites/MD001"),
        &cases.join("s1"),
    );
    copy_dir(
        &root.join("xtask/tests/corpus/markdown/sites/MD051"),
        &cases.join("s2"),
    );
    let private = temp("md-private");
    let args = HeldOutArgs {
        cases: Some(cases.clone()),
        archive: None,
        purpose: "public_control".to_owned(),
        private_dir: Some(private.clone()),
        kind: "check".to_owned(),
    };
    let (summary, code) = held_out::execute(&root, &args).expect("runner");
    assert_eq!(code, 0, "{summary}");
    let rows = summary["cases"].as_array().expect("cases");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r["outcome"] == "observed"), "{summary}");
    assert_eq!(rows[1]["witnesses"], 2, "MD051 has two witnesses");
    let text = serde_json::to_string(&summary).expect("json");
    assert!(
        !text.contains("Guide") && !text.contains("cafe"),
        "no page names or witness text"
    );
    std::fs::remove_dir_all(cases).expect("clean");
    std::fs::remove_dir_all(private).expect("clean");
}
