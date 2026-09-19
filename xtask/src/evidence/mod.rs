//! Evidence records (spec §11.3, §11.7.5): contributor claims, the tested
//! subject, the verification identity, observed check outcomes, and a
//! disposition that reports each acceptance predicate of §11.7.6 separately.
//!
//! Records are class `local` or `ci`, never `protected`: the producing
//! `xtask` is candidate code, and no producer is authenticated yet.

pub mod subject;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::cli::{CiArgs, Label};
use crate::error::{Context as _, Result};
use crate::lanes::{self, Outcome, RUN_DIR, Verdict};
use crate::policy::{Check, Lane, LoadedPolicy, POLICY_PATH};
use crate::tools::{Baseline, Probe};
use crate::util::{UtcTime, command_stdout, sha256_file};

pub use subject::Subject;

/// Where task records live (spec §11.7.11, stored as TOML).
pub const TASKS_DIR: &str = ".rha/tasks";

#[derive(Debug, Serialize)]
pub struct Record {
    pub schema_version: u32,
    pub record_kind: &'static str,
    pub evidence_class: &'static str,
    pub lane: String,
    pub started_at: String,
    pub finished_at: String,
    pub producer: Producer,
    pub change_claim: Value,
    pub artifact_identity: Subject,
    pub verification_identity: VerificationIdentity,
    pub observed_checks: Vec<CheckEntry>,
    pub evidence_inputs: EvidenceInputs,
    pub performance: Option<Value>,
    pub agent_context: Value,
    pub limits: Vec<String>,
    pub disposition: Disposition,
    #[serde(skip)]
    pub file_stem: String,
}

#[derive(Debug, Serialize)]
pub struct Producer {
    pub principal: String,
    pub accountable_to: Vec<String>,
    pub tool: String,
    pub tool_version: &'static str,
    pub tool_git_rev: String,
    pub tool_from_dirty_tree: bool,
}

#[derive(Debug, Serialize)]
pub struct VerificationIdentity {
    pub policy_path: &'static str,
    pub policy_digest: String,
    /// Where this run read its policy. Selection of `Policy(b)` from the base
    /// revision (§11.7.6) is not implemented, so this is always the candidate.
    pub policy_read_from: &'static str,
    pub base_revision: String,
    pub base_revision_source: String,
    /// The policy digest at the base revision: a digest, `absent`, or `unavailable`.
    pub base_policy_digest: String,
    pub toolchain: BTreeMap<String, String>,
    pub target: String,
    pub features: &'static str,
    pub profiles: &'static str,
    pub lockfile_sha256: Option<String>,
    pub configuration: Vec<Artifact>,
    pub instruction_sources: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckEntry {
    pub id: String,
    pub kind: String,
    pub argv: Vec<String>,
    pub required: bool,
    pub waivable: bool,
    pub params: BTreeMap<String, Value>,
    pub tool: Option<ToolUse>,
    pub outcome: Outcome,
    pub exit_status: Option<i32>,
    pub error_class: Option<String>,
    pub reason: Option<String>,
    pub started_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub selection_counts: Option<BTreeMap<String, u64>>,
    pub artifacts: Vec<Artifact>,
    pub limits: Vec<String>,
}

impl CheckEntry {
    #[must_use]
    pub fn new(check: &Check, params: BTreeMap<String, Value>, tool: Option<ToolUse>) -> Self {
        Self {
            id: check.id.clone(),
            kind: check.kind.clone(),
            argv: check.argv.clone(),
            required: check.required,
            waivable: check.waivable,
            params,
            tool,
            outcome: Outcome::NotRun,
            exit_status: None,
            error_class: None,
            reason: Some("not evaluated".to_owned()),
            started_at: None,
            duration_ms: None,
            selection_counts: None,
            artifacts: Vec::new(),
            limits: Vec::new(),
        }
    }

    pub fn set_verdict(&mut self, verdict: &Verdict) {
        self.outcome = verdict.outcome;
        self.exit_status = verdict.exit_status;
        self.error_class.clone_from(&verdict.error_class);
        self.reason.clone_from(&verdict.reason);
        self.selection_counts.clone_from(&verdict.selection_counts);
    }

    #[must_use]
    pub fn verdict(&self) -> Verdict {
        Verdict {
            outcome: self.outcome,
            exit_status: self.exit_status,
            error_class: self.error_class.clone(),
            reason: self.reason.clone(),
            selection_counts: self.selection_counts.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolUse {
    pub name: String,
    pub probe: Option<Probe>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Artifact {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct EvidenceInputs {
    pub fixtures: Vec<Artifact>,
    pub seeds: Value,
    pub environment: Value,
}

#[derive(Debug, Serialize)]
pub struct Disposition {
    /// `eligible`, `blocked`, or `not_evaluated` (lanes that are not acceptance lanes).
    pub eligibility: &'static str,
    pub predicates: Predicates,
    pub blocking: Vec<String>,
    pub acceptance: &'static str,
    pub exception: Option<Value>,
    pub acceptor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Predicates {
    pub authentic: Predicate,
    pub applicable: Predicate,
    pub complete: Predicate,
    pub passed: Predicate,
}

#[derive(Debug, Serialize)]
pub struct Predicate {
    pub holds: bool,
    pub reasons: Vec<String>,
}

impl Predicate {
    fn from_reasons(reasons: Vec<String>) -> Self {
        Self {
            holds: reasons.is_empty(),
            reasons,
        }
    }
}

/// Everything a record is built from.
pub struct RunContext<'a> {
    pub root: &'a Path,
    pub args: &'a CiArgs,
    pub lane_name: &'a str,
    pub lane: &'a Lane,
    pub loaded: &'a LoadedPolicy,
    pub baseline: &'a Baseline,
    pub started_at: UtcTime,
    pub finished_at: UtcTime,
    pub subject_before: &'a Subject,
    pub subject_after: &'a Subject,
    pub entries: Vec<CheckEntry>,
}

/// Builds the record for one lane run.
///
/// # Errors
/// Fails if an input file that exists cannot be read.
pub fn build(ctx: &RunContext<'_>) -> Result<Record> {
    let root = ctx.root;
    let policy = &ctx.loaded.policy;
    let evidence_class = match ctx.args.label {
        Label::Local => "local",
        Label::Ci => "ci",
    };
    let principal = ctx
        .args
        .principal
        .clone()
        .unwrap_or_else(|| match ctx.args.label {
            Label::Ci => "automation:github-actions".to_owned(),
            Label::Local => std::env::var("RHA_PRINCIPAL").unwrap_or_else(|_| "unknown".to_owned()),
        });

    let task_id = resolve_task(ctx.args, &ctx.subject_before.branch);
    let task = match &task_id {
        Some(id) => load_task(root, id)?,
        None => None,
    };

    let (base_revision, base_revision_source) = subject::base_revision(root);
    let base_policy = if base_revision == "unknown" {
        Err("base revision unknown".to_owned())
    } else {
        subject::digest_at(root, &base_revision, POLICY_PATH).map_err(|e| e.to_string())
    };
    let base_policy_digest = match &base_policy {
        Ok(Some(digest)) => digest.clone(),
        Ok(None) => "absent".to_owned(),
        Err(_) => "unavailable".to_owned(),
    };
    let verification_identity = verification_identity(
        root,
        ctx.loaded,
        task.as_ref(),
        (
            base_revision.clone(),
            base_revision_source,
            base_policy_digest,
        ),
    )?;

    let limits = vec![
        "policy read from the candidate working tree; Policy(b) selection from the base revision is not implemented (§11.7.6)".to_owned(),
        "the producing xtask is candidate code; this record is advisory, not protected evidence (§11.5)".to_owned(),
        "a timeout kills only the direct child process of a check".to_owned(),
    ];

    let mut record = Record {
        schema_version: 1,
        record_kind: "evidence",
        evidence_class,
        lane: ctx.lane_name.to_owned(),
        started_at: ctx.started_at.rfc3339(),
        finished_at: ctx.finished_at.rfc3339(),
        producer: Producer {
            principal,
            accountable_to: policy.authority.acceptance_authority.clone(),
            tool: format!("xtask ci --lane {}", ctx.lane_name),
            tool_version: env!("CARGO_PKG_VERSION"),
            tool_git_rev: ctx.subject_before.revision.clone(),
            tool_from_dirty_tree: ctx.subject_before.dirty,
        },
        change_claim: change_claim(task_id.as_deref(), task.as_ref()),
        artifact_identity: ctx.subject_before.clone(),
        verification_identity,
        observed_checks: ctx.entries.clone(),
        evidence_inputs: evidence_inputs(ctx)?,
        performance: None,
        agent_context: agent_context(task.as_ref()),
        limits,
        // Replaced below: the predicates read the rest of the record.
        disposition: Disposition {
            eligibility: "blocked",
            predicates: Predicates {
                authentic: Predicate::from_reasons(vec![]),
                applicable: Predicate::from_reasons(vec![]),
                complete: Predicate::from_reasons(vec![]),
                passed: Predicate::from_reasons(vec![]),
            },
            blocking: vec![],
            acceptance: "pending",
            exception: None,
            acceptor: None,
        },
        file_stem: file_stem(ctx.started_at, ctx.subject_before),
    };
    record.disposition = disposition(ctx, &record, &base_revision, &base_policy)?;
    Ok(record)
}

fn evidence_inputs(ctx: &RunContext<'_>) -> Result<EvidenceInputs> {
    Ok(EvidenceInputs {
        fixtures: existing_artifacts(
            ctx.root,
            &[ctx.lane.source_path(), "xtask/tests/corpus/manifest.toml"],
        )?,
        seeds: json!({
            "proptest_cases": ctx.loaded.policy.proptest.get(ctx.lane_name),
            "persisted_regressions": persisted_regressions(ctx.root),
        }),
        environment: environment(ctx.root, ctx.args.label),
    })
}

/// `<utc>-<first 12 hex of the revision>[-dirty]`.
fn file_stem(started_at: UtcTime, subject: &Subject) -> String {
    let short = subject.revision.get(..12).unwrap_or(&subject.revision);
    let dirty = if subject.dirty { "-dirty" } else { "" };
    format!("{}-{short}{dirty}", started_at.compact())
}

/// The verification-identity group; `base` is (revision, how it was found,
/// policy digest at that revision).
fn verification_identity(
    root: &Path,
    loaded: &LoadedPolicy,
    task: Option<&TaskRecord>,
    base: (String, String, String),
) -> Result<VerificationIdentity> {
    let mut instruction_sources = existing_artifacts(
        root,
        &[
            "AGENTS.md",
            "CONTRIBUTING.md",
            ".github/pull_request_template.md",
        ],
    )?;
    instruction_sources.extend(scoped_guides(root)?);
    if let Some(t) = task {
        instruction_sources.push(t.artifact.clone());
    }
    let (base_revision, base_revision_source, base_policy_digest) = base;
    Ok(VerificationIdentity {
        policy_path: POLICY_PATH,
        policy_digest: loaded.digest.clone(),
        policy_read_from: "candidate_worktree",
        base_revision,
        base_revision_source,
        base_policy_digest,
        toolchain: toolchain(root),
        target: host_target(root),
        features: "--all-features",
        profiles: "dev (clippy, doctest), test (nextest)",
        lockfile_sha256: optional_digest(&root.join("Cargo.lock"))?,
        configuration: existing_artifacts(
            root,
            &[
                "rust-toolchain.toml",
                "Cargo.toml",
                ".cargo/config.toml",
                "clippy.toml",
                ".config/nextest.toml",
                "deny.toml",
                "_typos.toml",
                "rha-baseline.json",
                "rha-crates.toml",
            ],
        )?,
        instruction_sources,
    })
}

fn disposition(
    ctx: &RunContext<'_>,
    record: &Record,
    base_revision: &str,
    base_policy: &std::result::Result<Option<String>, String>,
) -> Result<Disposition> {
    let policy = &ctx.loaded.policy;
    let authentic = if policy.authority.trusted_producers.is_empty() {
        vec![format!(
            "no trusted producers in policy: this {} record is unauthenticated",
            record.evidence_class
        )]
    } else {
        vec!["record signatures are not implemented (W16)".to_owned()]
    };

    let mut applicable = Vec::new();
    if ctx.subject_before != ctx.subject_after {
        applicable.push(format!(
            "the subject changed during the run: snapshot tree {} before, {} after",
            ctx.subject_before.snapshot_tree, ctx.subject_after.snapshot_tree
        ));
    }
    match base_policy {
        Ok(Some(digest)) if digest == &ctx.loaded.digest => {}
        Ok(Some(digest)) => applicable.push(format!(
            "the candidate's policy {} differs from the base policy {digest}",
            ctx.loaded.digest
        )),
        Ok(None) => applicable.push(format!(
            "base {base_revision} has no {POLICY_PATH}: no approved policy governs this change (bootstrap)"
        )),
        Err(e) => applicable.push(format!("base policy identity unverifiable: {e}")),
    }
    let snapshot = serde_json::to_value(record).context(|| "serializing the record".to_owned())?;
    for input in &policy.evidence.required_inputs {
        let pointer = format!("/{}", input.replace('.', "/"));
        match snapshot.pointer(&pointer) {
            None | Some(Value::Null) => {
                applicable.push(format!("required input {input} is missing"));
            }
            Some(Value::String(s)) if s.is_empty() || s == "unknown" => {
                applicable.push(format!("required input {input} is unknown"));
            }
            Some(_) => {}
        }
    }

    let mut complete = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in &record.observed_checks {
        if !seen.insert(entry.id.as_str()) {
            complete.push(format!("duplicate check id {}", entry.id));
        }
    }
    for check in &ctx.lane.checks {
        if !seen.contains(check.id.as_str()) {
            complete.push(format!("no entry for check {}", check.id));
        }
    }

    let required: BTreeSet<&str> = lanes::required_ids(ctx.lane).collect();
    let passed: Vec<String> = record
        .observed_checks
        .iter()
        .filter(|e| required.contains(e.id.as_str()) && e.outcome != Outcome::Passed)
        .map(|e| {
            format!(
                "{} {}: {}",
                e.id,
                e.outcome.as_str(),
                e.reason.as_deref().unwrap_or("")
            )
        })
        .collect();

    let predicates = Predicates {
        authentic: Predicate::from_reasons(authentic),
        applicable: Predicate::from_reasons(applicable),
        complete: Predicate::from_reasons(complete),
        passed: Predicate::from_reasons(passed),
    };
    let named = [
        ("Authentic", &predicates.authentic),
        ("Applicable", &predicates.applicable),
        ("Complete", &predicates.complete),
        ("Passed", &predicates.passed),
    ];
    let blocking: Vec<String> = named
        .iter()
        .flat_map(|(name, p)| p.reasons.iter().map(move |r| format!("{name}: {r}")))
        .collect();
    let eligibility = if !ctx.lane.acceptance {
        "not_evaluated"
    } else if blocking.is_empty() {
        "eligible"
    } else {
        "blocked"
    };
    Ok(Disposition {
        eligibility,
        predicates,
        blocking,
        acceptance: "pending",
        exception: None,
        acceptor: None,
    })
}

/// Writes `target/rha/evidence.json` and, with `dir`, a timestamped copy.
///
/// # Errors
/// Fails if a directory or file cannot be written.
pub fn write(root: &Path, record: &Record, dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut json =
        serde_json::to_string_pretty(record).context(|| "serializing the record".to_owned())?;
    json.push('\n');
    let run_dir = root.join(RUN_DIR);
    std::fs::create_dir_all(&run_dir).context(|| format!("creating {}", run_dir.display()))?;
    let latest = run_dir.join("evidence.json");
    std::fs::write(&latest, &json).context(|| format!("writing {}", latest.display()))?;
    let mut written = vec![latest];
    if let Some(dir) = dir {
        let dir = if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            root.join(dir)
        };
        std::fs::create_dir_all(&dir).context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{}.json", record.file_stem));
        std::fs::write(&path, &json).context(|| format!("writing {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

/// The task id for this run: `--task`, else a record directory named like
/// `CHG-000`, else a branch named like `chg/000-slug`.
#[must_use]
pub fn resolve_task(args: &CiArgs, branch: &str) -> Option<String> {
    if let Some(task) = &args.task {
        return Some(task.clone());
    }
    let from_dir = args
        .record
        .as_ref()
        .and_then(|d| d.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| is_change_id(n));
    if from_dir.is_some() {
        return from_dir;
    }
    let digits = branch.strip_prefix("chg/")?.split('-').next()?;
    let id = format!("CHG-{digits}");
    is_change_id(&id).then_some(id)
}

fn is_change_id(s: &str) -> bool {
    s.strip_prefix("CHG-")
        .is_some_and(|d| d.len() == 3 && d.bytes().all(|b| b.is_ascii_digit()))
}

/// A task record file and its parsed contents.
#[derive(Debug)]
pub struct TaskRecord {
    pub artifact: Artifact,
    pub table: toml::Table,
}

/// The unique `.rha/tasks/<id>*.toml` file, if there is exactly one.
///
/// # Errors
/// Fails if the file exists but cannot be read or parsed.
pub fn load_task(root: &Path, id: &str) -> Result<Option<TaskRecord>> {
    let dir = root.join(TASKS_DIR);
    let Ok(listing) = std::fs::read_dir(&dir) else {
        return Ok(None);
    };
    let mut matches: Vec<PathBuf> = listing
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "toml")
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(id))
        })
        .collect();
    if matches.len() != 1 {
        return Ok(None);
    }
    let path = matches.remove(0);
    let text = std::fs::read_to_string(&path).context(|| format!("reading {}", path.display()))?;
    let table: toml::Table =
        toml::from_str(&text).context(|| format!("parsing {}", path.display()))?;
    Ok(Some(TaskRecord {
        artifact: Artifact {
            path: lanes::relative(root, &path),
            sha256: sha256_file(&path)?,
        },
        table,
    }))
}

fn to_json(value: Option<&toml::Value>) -> Value {
    value
        .and_then(|v| serde_json::to_value(v).ok())
        .unwrap_or(Value::Null)
}

fn change_claim(task_id: Option<&str>, task: Option<&TaskRecord>) -> Value {
    let Some(task) = task else {
        return json!({ "task": task_id, "source": null, "note": "no task record found; claims unknown" });
    };
    let claim = task.table.get("claim").and_then(toml::Value::as_table);
    let field = |name: &str| to_json(claim.and_then(|c| c.get(name)));
    json!({
        "task": task_id,
        "source": task.artifact,
        "intent": to_json(task.table.get("intent")),
        "non_goals": to_json(task.table.get("non_goals")),
        "affected_components": field("affected_components"),
        "contract_changes": field("contract_changes"),
        "architecture_delta": field("architecture_delta"),
        "performance_impact": field("performance_impact"),
        "unresolved": field("unresolved"),
    })
}

fn agent_context(task: Option<&TaskRecord>) -> Value {
    let provenance = task.and_then(|t| t.table.get("provenance"));
    match provenance {
        Some(p) => json!({ "source": task.map(|t| &t.artifact), "provenance": to_json(Some(p)) }),
        None => {
            json!({ "model_id": "unknown", "harness": "unknown", "permissions": "unknown", "budgets": "unknown" })
        }
    }
}

fn optional_digest(path: &Path) -> Result<Option<String>> {
    if path.exists() {
        sha256_file(path).map(Some)
    } else {
        Ok(None)
    }
}

fn existing_artifacts(root: &Path, paths: &[&str]) -> Result<Vec<Artifact>> {
    let mut out = Vec::new();
    for rel in paths {
        let path = root.join(rel);
        if path.is_file() {
            out.push(Artifact {
                path: (*rel).to_owned(),
                sha256: sha256_file(&path)?,
            });
        }
    }
    Ok(out)
}

fn scoped_guides(root: &Path) -> Result<Vec<Artifact>> {
    let mut out = Vec::new();
    let Ok(listing) = std::fs::read_dir(root.join("crates")) else {
        return Ok(out);
    };
    let mut dirs: Vec<PathBuf> = listing
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .collect();
    dirs.sort();
    for dir in dirs {
        let guide = dir.join("AGENTS.md");
        if guide.is_file() {
            out.push(Artifact {
                path: lanes::relative(root, &guide),
                sha256: sha256_file(&guide)?,
            });
        }
    }
    Ok(out)
}

fn persisted_regressions(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(crates) = std::fs::read_dir(root.join("crates")) else {
        return out;
    };
    for krate in crates.filter_map(std::result::Result::ok) {
        let Ok(files) = std::fs::read_dir(krate.path().join("proptest-regressions")) else {
            continue;
        };
        for file in files.filter_map(std::result::Result::ok) {
            out.push(lanes::relative(root, &file.path()));
        }
    }
    out.sort();
    out
}

fn toolchain(root: &Path) -> BTreeMap<String, String> {
    let probes: [(&str, &[&str]); 4] = [
        ("rustc", &["rustc", "--version"]),
        ("cargo", &["cargo", "--version"]),
        ("clippy", &["cargo", "clippy", "--version"]),
        ("rustfmt", &["cargo", "fmt", "--version"]),
    ];
    probes
        .iter()
        .map(|(name, argv)| {
            let version = command_stdout(root, argv).unwrap_or_else(|_| "unknown".to_owned());
            ((*name).to_owned(), version)
        })
        .collect()
}

fn host_target(root: &Path) -> String {
    command_stdout(root, &["rustc", "-vV"])
        .ok()
        .and_then(|out| {
            out.lines()
                .find_map(|l| l.strip_prefix("host: ").map(str::to_owned))
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn environment(root: &Path, label: Label) -> Value {
    let os = command_stdout(root, &["uname", "-srm"])
        .unwrap_or_else(|_| std::env::consts::OS.to_owned());
    let ci = if label == Label::Ci {
        let keys = [
            "GITHUB_REPOSITORY",
            "GITHUB_WORKFLOW",
            "GITHUB_EVENT_NAME",
            "GITHUB_RUN_ID",
            "GITHUB_RUN_ATTEMPT",
            "GITHUB_SHA",
            "GITHUB_REF",
            "RHA_PR_HEAD_SHA",
            "RUNNER_OS",
            "RUNNER_ARCH",
            "ImageOS",
            "ImageVersion",
        ];
        let found: BTreeMap<&str, String> = keys
            .iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (*k, v)))
            .collect();
        json!(found)
    } else {
        Value::Null
    };
    json!({
        "os": os,
        "runner": if label == Label::Ci { "github-actions" } else { "local" },
        "ci": ci,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(task: Option<&str>, record: Option<&str>) -> CiArgs {
        CiArgs {
            lane: "L0".into(),
            label: Label::Local,
            record: record.map(PathBuf::from),
            only: None,
            print: false,
            task: task.map(str::to_owned),
            principal: None,
        }
    }

    #[test]
    fn task_comes_from_flag_then_record_dir_then_branch() {
        assert_eq!(
            resolve_task(
                &args(Some("CHG-007"), Some("evidence/CHG-001")),
                "chg/002-x"
            ),
            Some("CHG-007".into())
        );
        assert_eq!(
            resolve_task(&args(None, Some("evidence/CHG-001")), "chg/002-x"),
            Some("CHG-001".into())
        );
        assert_eq!(
            resolve_task(&args(None, Some("target/rha")), "chg/002-slug"),
            Some("CHG-002".into())
        );
        assert_eq!(resolve_task(&args(None, None), "main"), None);
        assert_eq!(resolve_task(&args(None, None), "chg/12-x"), None);
    }
}
