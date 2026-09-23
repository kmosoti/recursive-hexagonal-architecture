//! The registered P-B corpora are pinned by their tree digests: an edit to a
//! fixture or an expectation after registration fails here.

use std::collections::BTreeSet;
use std::path::{Component, Path};

fn tree_digest(root: &Path) -> String {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .expect("dir")
            .map(|e| e.expect("entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.is_dir() {
                walk(&path, base, out);
            } else if name != "registration.toml" && name != "GENERATOR-PROMPT.md" {
                let rel = path
                    .strip_prefix(base)
                    .expect("rel")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push(format!(
                    "{rel} {}\n",
                    xtask::util::sha256_hex(&std::fs::read(&path).expect("read"))
                ));
            }
        }
    }
    let mut lines = Vec::new();
    walk(root, root, &mut lines);
    lines.sort();
    xtask::util::sha256_hex(lines.concat().as_bytes())
}

fn pinned(rel: &str) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    let reg: toml::Value = toml::from_str(
        &std::fs::read_to_string(base.join("registration.toml")).expect("registration"),
    )
    .expect("toml");
    assert_eq!(
        tree_digest(&base),
        reg["tree_sha256"].as_str().expect("digest"),
        "{rel} changed after registration"
    );
}

fn module_registration() -> toml::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("xtask/tests/corpus/module/registration.toml");
    toml::from_str(&std::fs::read_to_string(path).expect("module registration"))
        .expect("module registration TOML")
}

fn safe_inventory_path(path: &str) -> bool {
    let candidate = Path::new(path);
    !path.is_empty()
        && !path.contains('\\')
        && candidate.is_relative()
        && candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn lower_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn walk_module_payloads(
    directory: &Path,
    base: &Path,
    payloads: &mut BTreeSet<String>,
    bytes: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("reading {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("reading directory entry: {error}"))?
            .path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("reading metadata for {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symlink in module inventory: {}", path.display()));
        }
        if metadata.is_dir() {
            walk_module_payloads(&path, base, payloads, bytes)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(format!(
                "non-file module inventory entry: {}",
                path.display()
            ));
        }
        let relative = path
            .strip_prefix(base)
            .map_err(|error| format!("relative module path: {error}"))?
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "registration.toml" || relative == "SHA256SUMS" {
            continue;
        }
        *bytes += metadata.len();
        payloads.insert(relative);
    }
    Ok(())
}

fn verify_module_inventory(root: &Path) -> Result<(), String> {
    let sums_path = root.join("SHA256SUMS");
    let sums_bytes =
        std::fs::read(&sums_path).map_err(|error| format!("reading SHA256SUMS: {error}"))?;
    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(root.join("registration.toml"))
            .map_err(|error| format!("reading registration: {error}"))?,
    )
    .map_err(|error| format!("parsing registration: {error}"))?;
    let registered_sums_digest = registration["content_sha256"]
        .as_str()
        .ok_or_else(|| "registration.content_sha256 is absent".to_owned())?;
    if xtask::util::sha256_hex(&sums_bytes) != registered_sums_digest {
        return Err("registration.content_sha256 does not match SHA256SUMS".to_owned());
    }

    let text = std::str::from_utf8(&sums_bytes)
        .map_err(|error| format!("SHA256SUMS is not UTF-8: {error}"))?;
    let mut listed = BTreeSet::new();
    let mut previous = None;
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 66 || bytes[64] != b' ' || bytes[65] != b' ' {
            return Err(format!("malformed SHA256SUMS line: {line:?}"));
        }
        let digest = &line[..64];
        let relative = &line[66..];
        if !lower_sha256(digest) || !safe_inventory_path(relative) {
            return Err(format!("unsafe SHA256SUMS line: {line:?}"));
        }
        // The registered generator sorts pathlib paths by their components.
        if previous.is_some_and(|old: &str| old.split('/').cmp(relative.split('/')).is_ge()) {
            return Err(format!("SHA256SUMS is not strictly sorted at {relative}"));
        }
        previous = Some(relative);
        if !listed.insert(relative.to_owned()) {
            return Err(format!("duplicate SHA256SUMS path: {relative}"));
        }

        let path = root.join(relative);
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("reading listed payload {relative}: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!("listed payload is not a durable file: {relative}"));
        }
        let actual = xtask::util::sha256_file(&path)
            .map_err(|error| format!("hashing listed payload {relative}: {error}"))?;
        if actual != digest {
            return Err(format!("payload digest mismatch: {relative}"));
        }
    }

    let mut actual = BTreeSet::new();
    let mut payload_bytes = 0;
    walk_module_payloads(root, root, &mut actual, &mut payload_bytes)?;
    if actual != listed {
        return Err(format!(
            "SHA256SUMS inventory differs from durable payload set: listed {}, actual {}",
            listed.len(),
            actual.len()
        ));
    }
    if registration["content_files"].as_integer()
        != Some(i64::try_from(actual.len()).map_err(|_| "payload count overflow".to_owned())?)
    {
        return Err("registration.content_files does not match payload count".to_owned());
    }
    if registration["content_bytes"].as_integer()
        != Some(i64::try_from(payload_bytes).map_err(|_| "payload byte count overflow".to_owned())?)
    {
        return Err("registration.content_bytes does not match payload bytes".to_owned());
    }
    Ok(())
}

#[test]
fn the_verifier_corpus_is_exactly_what_was_registered() {
    pinned("tools/rha-verifier/tests/corpus");
}

#[test]
fn the_record_schema_corpus_is_exactly_what_was_registered() {
    pinned("xtask/tests/corpus/record-schema");
}

#[test]
fn the_verifier_shape_corpus_is_exactly_what_was_registered() {
    pinned("xtask/tests/corpus/verifier-shape");
}

#[test]
fn the_record_corpus_is_exactly_what_was_registered() {
    pinned("xtask/tests/corpus/records");
}

#[test]
fn the_module_corpus_inventory_is_complete_and_pinned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("xtask/tests/corpus/module");
    verify_module_inventory(&root).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn the_module_registration_pins_generator_reference_contract_and_counts() {
    let registration = module_registration();
    assert_eq!(registration["model"].as_str(), Some("gpt-6-astra"));
    assert_eq!(registration["effort"].as_str(), Some("xhigh"));
    assert_eq!(registration["seed"].as_integer(), Some(20260922));
    assert_eq!(
        registration["rng"].as_str(),
        Some("SplitMix64; constants and modulo sampling in generate.py")
    );
    assert_eq!(
        registration["generator_sha256"].as_str(),
        Some("f215411fd95a3c712af71f1ddcf331489fa8852c6b3eda210012523111bab3ae")
    );
    assert_eq!(
        registration["reference_sha256"].as_str(),
        Some("dd4ce1ce37e619b054860c9428f48b0cdcc3e4672d166cbd7803c41ed85d36fa")
    );
    assert_eq!(
        registration["module_check_contract_sha256"].as_str(),
        Some("3f0b6d3c4c647d9210ea5e86517b15ffb2e7126eb08add2b33bbf7ef7751133e")
    );
    assert_eq!(
        registration["random_content_sha256"].as_str(),
        Some("e0f79fb8a425897a3bc03f4bae055d4c6890996af0a9687d859a4bbf063fa68a")
    );
    assert_eq!(
        registration["headline_content_sha256"].as_str(),
        Some("d816d2bd6edc5a07c7ad1ae0f7b48bd104cf92ddd1296e9095278f1ca2465a1c")
    );

    let counts = &registration["counts"];
    for (name, expected) in [
        ("headline_edges", 41),
        ("headline_fixtures", 25),
        ("headline_include_fragments", 1),
        ("headline_limitations", 4),
        ("headline_rust_files", 27),
        ("random_cases", 256),
        ("random_edges", 5229),
        ("random_heuristic_edges", 116),
        ("random_limitations", 0),
        ("random_rust_files", 1228),
        ("random_source_map_files", 1740),
        ("random_test_edges", 512),
        ("optional_cycles", 64),
        ("optional_subsumed_undeclared_edges", 64),
        ("optional_top_level_findings", 192),
        ("optional_top_level_undeclared_edges", 64),
        ("optional_total_undeclared_edge_facts", 128),
        ("preserved_original_artifacts", 672),
    ] {
        assert_eq!(counts[name].as_integer(), Some(expected), "counts.{name}");
    }
}

#[test]
fn the_module_inventory_checker_rejects_changed_and_missing_payloads() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "rha-module-inventory-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("miniature inventory directory");

    let payload = root.join("payload.txt");
    std::fs::write(&payload, b"original\n").expect("miniature payload");
    let payload_digest = xtask::util::sha256_hex(b"original\n");
    let sums = format!("{payload_digest}  payload.txt\n");
    std::fs::write(root.join("SHA256SUMS"), &sums).expect("miniature SHA256SUMS");
    let sums_digest = xtask::util::sha256_hex(sums.as_bytes());
    std::fs::write(
        root.join("registration.toml"),
        format!(
            "content_sha256 = \"{sums_digest}\"\ncontent_files = 1\ncontent_bytes = {}\n",
            b"original\n".len()
        ),
    )
    .expect("miniature registration");
    assert!(verify_module_inventory(&root).is_ok());

    std::fs::write(&payload, b"changed\n").expect("changed miniature payload");
    assert!(verify_module_inventory(&root).is_err());

    std::fs::write(&payload, b"original\n").expect("restore miniature payload");
    std::fs::remove_file(&payload).expect("remove miniature payload");
    assert!(verify_module_inventory(&root).is_err());

    let _ = std::fs::remove_dir_all(&root);
}
