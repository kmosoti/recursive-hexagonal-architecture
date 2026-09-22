//! `cargo xtask corpus run --level markdown`: the registered adversarial
//! markdown corpus (P-A stages 1 and 4), graded through the real binary
//! exactly as registered, with an evidence record.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::{Context as _, Error, Result};
use crate::util::{UtcTime, command, git_identity, sha256_hex};

/// Where the corpus lives.
pub const ROOT: &str = "xtask/tests/corpus/markdown";

/// The digest the registration pins: sorted `<path> <sha256>` lines over
/// `sites/`.
///
/// # Errors
/// On an unreadable file.
pub fn tree_digest(sites: &Path) -> Result<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<()> {
        for entry in std::fs::read_dir(dir).context(|| format!("reading {}", dir.display()))? {
            let path = entry.context(|| "reading an entry".to_owned())?.path();
            if path.is_dir() {
                walk(&path, base, out)?;
            } else {
                let rel = path
                    .strip_prefix(base)
                    .map_err(|e| Error::new(e.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes =
                    std::fs::read(&path).context(|| format!("reading {}", path.display()))?;
                out.push(format!("{rel} {}\n", sha256_hex(&bytes)));
            }
        }
        Ok(())
    }
    let mut lines = Vec::new();
    walk(sites, sites, &mut lines)?;
    lines.sort();
    Ok(sha256_hex(lines.concat().as_bytes()))
}

fn multiset(v: &Value) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for w in v.as_array().into_iter().flatten() {
        *m.entry(w.to_string()).or_default() += 1;
    }
    m
}

/// Runs every registered site and writes the record. Exit 1 on any site not
/// graded as registered, and 2 if the corpus differs from its registration.
///
/// # Errors
/// On unreadable files, a failed build, or evidence I/O.
pub fn run(root: &Path, evidence: &Path) -> Result<u8> {
    let base = root.join(ROOT);
    let registration: toml::Value = toml::from_str(
        &std::fs::read_to_string(base.join("registration.toml"))
            .context(|| "reading the registration".to_owned())?,
    )
    .context(|| "parsing the registration".to_owned())?;
    let registered = registration
        .get("tree_sha256")
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let actual = tree_digest(&base.join("sites"))?;
    if actual != registered {
        eprintln!("corpus differs from its registration: sha256 {actual} != {registered}");
        return Ok(2);
    }
    let build = command("cargo")
        .args(["build", "-q", "-p", "app-cli"])
        .current_dir(root)
        .status()
        .context(|| "building rhawiki".to_owned())?;
    if !build.success() {
        return Err(Error::new("rhawiki did not build"));
    }
    let binary = root.join("target/debug/rhawiki");
    let mut sites: Vec<_> = std::fs::read_dir(base.join("sites"))
        .context(|| "reading sites".to_owned())?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .collect();
    sites.sort();
    let mut cases = Vec::new();
    for site in &sites {
        let expected: Value = serde_json::from_slice(
            &std::fs::read(site.join("EXPECTED.json"))
                .context(|| "reading an expectation".to_owned())?,
        )
        .context(|| "parsing an expectation".to_owned())?;
        let out = command(&binary.to_string_lossy())
            .args(["check", "--format", "json", "--root"])
            .arg(site)
            .output()
            .context(|| "running rhawiki".to_owned())?;
        let got: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        let want = multiset(&expected["witnesses"]);
        let have = multiset(&got["witnesses"]);
        let exit_ok = out.status.code() == Some(i32::from(!want.is_empty()));
        let missing: Vec<&String> = want
            .keys()
            .filter(|k| have.get(*k) < want.get(*k))
            .collect();
        let extra: Vec<&String> = have
            .keys()
            .filter(|k| want.get(*k) < have.get(*k))
            .collect();
        cases.push(json!({
            "site": site.file_name().map(|n| n.to_string_lossy().into_owned()),
            "expected_witnesses": expected["witnesses"].as_array().map_or(0, Vec::len),
            "observed_witnesses": got["witnesses"].as_array().map_or(0, Vec::len),
            "exit_status": out.status.code(),
            "missing": missing, "extra": extra,
            "passed": missing.is_empty() && extra.is_empty() && exit_ok,
        }));
    }
    let passed = cases.iter().filter(|c| c["passed"] == true).count();
    let now = UtcTime::now();
    let subject = crate::evidence::subject::capture(root, &root.join("target/rha/md-subject"))?;
    let identity = git_identity(root);
    let record = json!({
        "schema_version": 1, "kind": "markdown_corpus", "evidence_class": "local", "advisory": true,
        "created_at": now.rfc3339(), "artifact_identity": subject,
        "product": {"binary": "rhawiki", "git_rev": identity.as_ref().map(|i| i.revision.clone()), "git_dirty": identity.as_ref().map(|i| i.dirty)},
        "registration": {"path": format!("{ROOT}/registration.toml"), "tree_sha256": registered, "generator": registration.get("generator").map(|g| g.to_string())},
        "grading": "exact multiset of witnesses, every key equal, no extras; exit 1 iff any witness",
        "summary": {"sites": cases.len(), "passed": passed, "failed": cases.len() - passed, "outcome": if passed == cases.len() { "passed" } else { "failed" }},
        "cases": cases,
    });
    let dir = if evidence.ends_with("h4-crate") {
        root.join("evidence/md-corpus")
    } else {
        root.join(evidence)
    };
    std::fs::create_dir_all(&dir).context(|| "creating the evidence directory".to_owned())?;
    let path = dir.join(format!(
        "{}-{}{}.json",
        now.compact(),
        &subject.revision[..12],
        if subject.dirty { "-dirty" } else { "" }
    ));
    let text = serde_json::to_vec_pretty(&record).context(|| "serializing".to_owned())?;
    std::fs::write(&path, text).context(|| format!("writing {}", path.display()))?;
    println!(
        "markdown corpus: {passed}/{} sites graded as registered; evidence {}",
        record["summary"]["sites"],
        path.strip_prefix(root).unwrap_or(&path).display()
    );
    Ok(u8::from(
        passed
            != record["summary"]["sites"]
                .as_u64()
                .map_or(0, |n| usize::try_from(n).unwrap_or(0)),
    ))
}
