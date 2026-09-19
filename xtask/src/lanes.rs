//! Runs one lane of `.rha/policy.toml` and classifies each check's outcome.
//!
//! Classification is a pure function of the check and an [`Observation`], so
//! the rules that turn exit statuses and reports into the five outcome states
//! (spec §11.4) are tested without running any tool (spec §12.3).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

use crate::architecture::EXIT_NOT_RUN;
use crate::cli::{CiArgs, Label};
use crate::error::{Context as _, Error, Result};
use crate::evidence::{self, Artifact, CheckEntry, ToolUse};
use crate::policy::{self, Check, Lane, Validity};
use crate::tools::{self, Probe, SELF_TOOL};
use crate::util::{UtcTime, sha256_file};

/// Where logs, reports, and the latest record of a run are written.
pub const RUN_DIR: &str = "target/rha";

/// The five outcome states of spec §11.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    NotRun,
    NotApplicable,
    Inconclusive,
}

impl Outcome {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NotRun => "not_run",
            Self::NotApplicable => "not_applicable",
            Self::Inconclusive => "inconclusive",
        }
    }
}

/// How a spawned command ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ended {
    Exited(i32),
    Signalled,
    TimedOut,
    NotFound,
    SpawnFailed(String),
}

/// What classification needs from one run of a check's command.
#[derive(Debug, Clone)]
pub struct Observation {
    pub ended: Ended,
    pub stdout: String,
    pub stderr: String,
    /// The check's report file as read after the run; `None` if absent.
    pub report: Option<String>,
}

/// An outcome with the facts that justify it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub outcome: Outcome,
    pub exit_status: Option<i32>,
    pub error_class: Option<String>,
    pub reason: Option<String>,
    pub selection_counts: Option<BTreeMap<String, u64>>,
}

impl Verdict {
    fn passed(code: i32, counts: Option<BTreeMap<String, u64>>) -> Self {
        Self {
            outcome: Outcome::Passed,
            exit_status: Some(code),
            error_class: None,
            reason: None,
            selection_counts: counts,
        }
    }

    fn failed(code: Option<i32>, class: &str, reason: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::Failed,
            exit_status: code,
            error_class: Some(class.to_owned()),
            reason: Some(reason.into()),
            selection_counts: None,
        }
    }

    fn not_run(reason: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::NotRun,
            exit_status: None,
            error_class: None,
            reason: Some(reason.into()),
            selection_counts: None,
        }
    }
}

/// Decides whether a check runs at all. `Some` is a `not_run` verdict.
#[must_use]
pub fn gate(check: &Check, selected: bool, probe: Option<&Probe>) -> Option<Verdict> {
    if check.argv.is_empty() {
        return Some(Verdict::not_run(
            "no_approved_command: the policy lists this check without a command",
        ));
    }
    if !selected {
        return Some(Verdict::not_run("not_selected: excluded by --only"));
    }
    let tool = check.tool.as_deref().unwrap_or(SELF_TOOL);
    match probe {
        None | Some(Probe::Ready { .. }) => None,
        Some(Probe::Missing { detail }) => {
            Some(Verdict::not_run(format!("tool_missing: {tool}: {detail}")))
        }
        Some(Probe::Mismatch {
            installed,
            required,
        }) => Some(Verdict::not_run(format!(
            "version_mismatch: {tool} installed {installed}, required {required} (rha-baseline.json)"
        ))),
        Some(Probe::Unlisted) => Some(Verdict::not_run(format!(
            "tool_unlisted: {tool} has no required version in rha-baseline.json"
        ))),
    }
}

/// Classifies one run. Exit status zero is necessary but not sufficient for
/// `passed`: the check's validity criterion must also hold.
#[must_use]
pub fn classify(check: &Check, obs: &Observation) -> Verdict {
    let code = match &obs.ended {
        Ended::Exited(code) => *code,
        Ended::NotFound => {
            let program = check.argv.first().map_or("", String::as_str);
            return Verdict::not_run(format!("tool_missing: `{program}` not found when spawned"));
        }
        Ended::SpawnFailed(e) => {
            return Verdict::failed(None, "tool_error", format!("spawn failed: {e}"));
        }
        Ended::TimedOut => {
            let limit = check.timeout_secs.unwrap_or(0);
            return Verdict::failed(
                None,
                "timeout",
                format!("no exit within {limit} s; the process was killed"),
            );
        }
        Ended::Signalled => return Verdict::failed(None, "tool_error", "terminated by a signal"),
    };
    let Some(validity) = check.validity else {
        return Verdict::failed(
            Some(code),
            "config_error",
            "the policy gives this check no validity criterion",
        );
    };
    match validity {
        Validity::ArchitectureReportPassed => classify_architecture(code, obs.report.as_deref()),
        _ if code != 0 => Verdict::failed(
            Some(code),
            failure_class(check, &obs.stderr),
            format!("exited with {code}"),
        ),
        Validity::ExitZero => Verdict::passed(0, None),
        Validity::JunitTestsSelected => junit_verdict(obs.report.as_deref()),
        Validity::DoctestHarnessRan => doctest_verdict(&obs.stdout),
    }
}

/// The architecture report and the exit status must agree. A disagreement is
/// an invalid report, never a pass.
fn classify_architecture(code: i32, report: Option<&str>) -> Verdict {
    let parsed: Option<Value> = report.and_then(|r| serde_json::from_str(r).ok());
    let summary = parsed.as_ref().and_then(|v| v.get("summary"));
    let outcome = summary
        .and_then(|s| s.get("outcome"))
        .and_then(Value::as_str);
    let reason = summary
        .and_then(|s| s.get("reason"))
        .and_then(Value::as_str);
    match (code, outcome) {
        (0, Some("passed")) => {
            let mut counts = BTreeMap::new();
            for key in ["errors", "warnings", "crates", "edges"] {
                if let Some(n) = summary.and_then(|s| s.get(key)).and_then(Value::as_u64) {
                    counts.insert(key.to_owned(), n);
                }
            }
            Verdict::passed(0, Some(counts))
        }
        (EXIT_NOT_RUN, Some("not_run")) => Verdict::not_run(format!(
            "{} (xtask exit {EXIT_NOT_RUN})",
            reason.unwrap_or("the report gives no reason")
        )),
        (1, _) => Verdict::failed(
            Some(1),
            "violation",
            "architecture findings; see the report",
        ),
        (2, _) => Verdict::failed(Some(2), "config_error", "usage or configuration error"),
        (3, _) => Verdict::failed(
            Some(3),
            "tool_error",
            "environment failure (cargo metadata)",
        ),
        (0 | EXIT_NOT_RUN, other) => Verdict::failed(
            Some(code),
            "invalid_report",
            format!(
                "exit {code} but report outcome is {}",
                other.unwrap_or("missing")
            ),
        ),
        (other, _) => Verdict::failed(Some(other), "nonzero_exit", format!("exited with {other}")),
    }
}

/// Network failures of dependency checks are named so they are not mistaken
/// for policy violations. The match is on message text and is heuristic.
fn failure_class(check: &Check, stderr: &str) -> &'static str {
    const NETWORK_HINTS: [&str; 5] = [
        "failed to fetch",
        "could not resolve host",
        "network is unreachable",
        "connection refused",
        "operation timed out",
    ];
    let lower = stderr.to_lowercase();
    if check.kind == "dependency" && NETWORK_HINTS.iter().any(|hint| lower.contains(hint)) {
        "network"
    } else {
        "nonzero_exit"
    }
}

fn junit_verdict(report: Option<&str>) -> Verdict {
    let Some(xml) = report else {
        return Verdict::failed(
            Some(0),
            "invalid_report",
            "exit 0 but no junit report was written",
        );
    };
    let Some(counts) = junit_counts(xml) else {
        return Verdict::failed(
            Some(0),
            "invalid_report",
            "junit report has no <testsuites tests=\"…\">",
        );
    };
    if counts.tests == 0 {
        return Verdict::failed(Some(0), "validity", "no tests selected");
    }
    if counts.failures + counts.errors > 0 {
        return Verdict::failed(
            Some(0),
            "validity",
            "exit 0 but the junit report lists failures or errors",
        );
    }
    Verdict::passed(
        0,
        Some(BTreeMap::from([
            ("tests".to_owned(), counts.tests),
            ("failures".to_owned(), counts.failures),
            ("errors".to_owned(), counts.errors),
        ])),
    )
}

#[derive(Debug, PartialEq, Eq)]
struct JunitCounts {
    tests: u64,
    failures: u64,
    errors: u64,
}

/// Reads the counts on the `<testsuites>` element. Deliberately minimal: it
/// reads three attributes of one element, which is all validity needs.
fn junit_counts(xml: &str) -> Option<JunitCounts> {
    let start = xml.find("<testsuites")?;
    let end = start + xml[start..].find('>')?;
    let tag = &xml[start..end];
    let attr = |name: &str| -> Option<u64> {
        let key = format!(" {name}=\"");
        let from = tag.find(&key)? + key.len();
        let to = from + tag[from..].find('"')?;
        tag[from..to].parse().ok()
    };
    Some(JunitCounts {
        tests: attr("tests")?,
        failures: attr("failures").unwrap_or(0),
        errors: attr("errors").unwrap_or(0),
    })
}

fn doctest_verdict(stdout: &str) -> Verdict {
    let mut runs = 0_u64;
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    for line in stdout.lines().filter(|l| l.starts_with("test result:")) {
        runs += 1;
        for part in line.split(';') {
            let mut words = part.split_whitespace().rev();
            let (Some(label), Some(number)) = (words.next(), words.next()) else {
                continue;
            };
            if let Ok(n) = number.parse::<u64>()
                && matches!(label, "passed" | "failed" | "ignored")
            {
                *totals.entry(label.to_owned()).or_default() += n;
            }
        }
    }
    if runs == 0 {
        return Verdict::failed(
            Some(0),
            "validity",
            "no doc-test result lines in the output",
        );
    }
    if totals.get("failed").copied().unwrap_or(0) > 0 {
        return Verdict::failed(
            Some(0),
            "validity",
            "exit 0 but doc-test results list failures",
        );
    }
    totals.insert("harness_runs".to_owned(), runs);
    Verdict::passed(0, Some(totals))
}

/// The process exit status of `xtask ci`. It reports whether anything
/// failed; it is not eligibility, which only the record states.
#[must_use]
pub fn exit_code(verdicts: &[Verdict], label: Label) -> u8 {
    let failed = verdicts
        .iter()
        .any(|v| matches!(v.outcome, Outcome::Failed | Outcome::Inconclusive));
    let environment_gap = label == Label::Ci
        && verdicts.iter().any(|v| {
            v.outcome == Outcome::NotRun
                && v.reason.as_deref().is_some_and(|r| {
                    ["tool_missing", "version_mismatch", "tool_unlisted"]
                        .iter()
                        .any(|p| r.starts_with(p))
                })
        });
    u8::from(failed || environment_gap)
}

/// Entry point of `cargo xtask ci`.
///
/// # Errors
/// Fails when the policy or inventory is unusable or a record cannot be written.
pub fn run(root: &Path, args: &CiArgs) -> Result<u8> {
    let loaded = policy::load(root)?;
    let problems = policy::validate(&loaded.policy);
    if !problems.is_empty() {
        return Err(Error::new(format!(
            "unusable policy: {}",
            problems.join("; ")
        )));
    }
    let lane = loaded
        .policy
        .lanes
        .get(&args.lane)
        .ok_or_else(|| Error::new(format!("policy has no lane {}", args.lane)))?;
    if args.print {
        for check in &lane.checks {
            if check.argv.is_empty() {
                println!("{}: (no approved command)", check.id);
            } else {
                println!("{}", check.argv.join(" "));
            }
        }
        return Ok(0);
    }
    if let Some(only) = &args.only
        && !lane.checks.iter().any(|c| &c.id == only)
    {
        return Err(Error::new(format!(
            "lane {} has no check {only}",
            args.lane
        )));
    }

    let baseline = tools::load(root)?;
    let run_dir = root.join(RUN_DIR);
    let log_dir = run_dir.join("logs").join(&args.lane);
    if log_dir.exists() {
        std::fs::remove_dir_all(&log_dir).context(|| format!("clearing {}", log_dir.display()))?;
    }
    std::fs::create_dir_all(&log_dir).context(|| format!("creating {}", log_dir.display()))?;

    let started_at = UtcTime::now();
    let subject_before = evidence::subject::capture(root, &run_dir)?;
    let mut entries = Vec::new();
    for check in &lane.checks {
        entries.push(run_check(
            root,
            &loaded.policy,
            check,
            &baseline,
            args,
            &log_dir,
        )?);
    }
    let subject_after = evidence::subject::capture(root, &run_dir)?;
    let finished_at = UtcTime::now();

    let record = evidence::build(&evidence::RunContext {
        root,
        args,
        lane_name: &args.lane,
        lane,
        loaded: &loaded,
        baseline: &baseline,
        started_at,
        finished_at,
        subject_before: &subject_before,
        subject_after: &subject_after,
        entries,
    })?;
    let written = evidence::write(root, &record, args.record.as_deref())?;

    print_summary(&record, &written);
    if std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true") {
        github_annotations(&record);
    }
    let verdicts: Vec<Verdict> = record
        .observed_checks
        .iter()
        .map(CheckEntry::verdict)
        .collect();
    Ok(exit_code(&verdicts, args.label))
}

fn run_check(
    root: &Path,
    policy: &policy::Policy,
    check: &Check,
    baseline: &tools::Baseline,
    args: &CiArgs,
    log_dir: &Path,
) -> Result<CheckEntry> {
    let selected = args.only.as_ref().is_none_or(|only| only == &check.id);
    let tool_name = check.tool.clone();
    let probe = match tool_name.as_deref() {
        None | Some(SELF_TOOL) => None,
        Some(name) => Some(
            baseline
                .find(name)
                .map_or(Probe::Unlisted, |spec| tools::probe(root, spec)),
        ),
    };
    let mut params = BTreeMap::new();
    let proptest_cases = check
        .proptest
        .as_ref()
        .and_then(|key| policy.proptest.get(key).copied());
    if let Some(cases) = proptest_cases {
        params.insert("proptest_cases".to_owned(), Value::from(cases));
    }
    if let Some(timeout) = check.timeout_secs {
        params.insert("timeout_secs".to_owned(), Value::from(timeout));
    }
    let tool = tool_name.map(|name| ToolUse {
        name,
        probe: probe.clone(),
    });
    let mut entry = CheckEntry::new(check, params, tool);

    if let Some(verdict) = gate(check, selected, probe.as_ref()) {
        entry.set_verdict(&verdict);
        return Ok(entry);
    }

    let report_path = check.report.as_ref().map(|r| root.join(r));
    if let Some(path) = &report_path
        && path.exists()
    {
        std::fs::remove_file(path)
            .context(|| format!("removing stale report {}", path.display()))?;
    }
    let stdout_path = log_dir.join(format!("{}.stdout", check.id));
    let stderr_path = log_dir.join(format!("{}.stderr", check.id));
    let started = UtcTime::now();
    let clock = Instant::now();
    let ended = spawn_and_wait(root, check, proptest_cases, &stdout_path, &stderr_path)?;
    let duration_ms = u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX);

    let stdout = read_lossy(&stdout_path);
    let stderr = read_lossy(&stderr_path);
    let report = report_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());
    let verdict = classify(
        check,
        &Observation {
            ended,
            stdout,
            stderr,
            report,
        },
    );
    entry.set_verdict(&verdict);
    entry.started_at = Some(started.rfc3339());
    entry.duration_ms = Some(duration_ms);
    let mut artifact_paths: Vec<PathBuf> = vec![stdout_path, stderr_path];
    if let Some(path) = report_path.filter(|p| p.exists()) {
        artifact_paths.push(path);
    }
    for path in artifact_paths {
        entry.artifacts.push(Artifact {
            path: relative(root, &path),
            sha256: sha256_file(&path)?,
        });
    }
    Ok(entry)
}

fn spawn_and_wait(
    root: &Path,
    check: &Check,
    proptest_cases: Option<u32>,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<Ended> {
    let (program, args) = check
        .argv
        .split_first()
        .ok_or_else(|| Error::new(format!("{} has no command", check.id)))?;
    let stdout =
        File::create(stdout_path).context(|| format!("creating {}", stdout_path.display()))?;
    let stderr =
        File::create(stderr_path).context(|| format!("creating {}", stderr_path.display()))?;
    let mut command = crate::util::command(program);
    command
        .args(args)
        .current_dir(root)
        .stdout(stdout)
        .stderr(stderr)
        .env("CARGO_TERM_COLOR", "never");
    if let Some(cases) = proptest_cases {
        command.env("PROPTEST_CASES", cases.to_string());
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Ended::NotFound),
        Err(e) => return Ok(Ended::SpawnFailed(e.to_string())),
    };
    let limit = Duration::from_secs(check.timeout_secs.unwrap_or(3600));
    let clock = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.code().map_or(Ended::Signalled, Ended::Exited)),
            Ok(None) if clock.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(Ended::TimedOut);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Ok(Ended::SpawnFailed(e.to_string())),
        }
    }
}

fn read_lossy(path: &Path) -> String {
    std::fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// `path` relative to `root`, with `/` separators.
#[must_use]
pub fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn print_summary(record: &evidence::Record, written: &[PathBuf]) {
    println!();
    println!("lane {} ({}):", record.lane, record.evidence_class);
    for c in &record.observed_checks {
        let secs = c
            .duration_ms
            .map(|ms| format!("{:>6.1}s", Duration::from_millis(ms).as_secs_f64()))
            .unwrap_or_default();
        let detail = c.reason.as_deref().unwrap_or("");
        println!(
            "  {:<18} {:<14} {secs:>7}  {detail}",
            c.id,
            c.outcome.as_str()
        );
    }
    let d = &record.disposition;
    println!("eligibility: {}", d.eligibility);
    for reason in &d.blocking {
        println!("  - {reason}");
    }
    for path in written {
        println!("record: {}", path.display());
    }
}

fn github_annotations(record: &evidence::Record) {
    for c in &record.observed_checks {
        let reason = c.reason.as_deref().unwrap_or("");
        match c.outcome {
            Outcome::Failed | Outcome::Inconclusive => {
                println!("::error title={} {}::{reason}", c.id, c.outcome.as_str());
            }
            Outcome::NotRun => println!("::warning title={} not_run::{reason}", c.id),
            Outcome::Passed | Outcome::NotApplicable => {}
        }
    }
    let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") else {
        return;
    };
    let mut md = format!(
        "### Lane {} ({})\n\n| Check | Outcome | Reason |\n| --- | --- | --- |\n",
        record.lane, record.evidence_class
    );
    for c in &record.observed_checks {
        let _ = writeln!(
            md,
            "| `{}` | {} | {} |",
            c.id,
            c.outcome.as_str(),
            c.reason.as_deref().unwrap_or("")
        );
    }
    let _ = writeln!(
        md,
        "\n**Eligibility:** {}\n",
        record.disposition.eligibility
    );
    for reason in &record.disposition.blocking {
        let _ = writeln!(md, "- {reason}");
    }
    let _ = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(summary_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, md.as_bytes()));
}

/// Checks of `lane` that are required for acceptance.
pub fn required_ids(lane: &Lane) -> impl Iterator<Item = &str> {
    lane.checks
        .iter()
        .filter(|c| c.required)
        .map(|c| c.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(validity: Validity) -> Check {
        Check {
            id: "L0.x".into(),
            kind: "test".into(),
            argv: vec!["cargo".into(), "x".into()],
            tool: Some("cargo-x".into()),
            validity: Some(validity),
            report: None,
            proptest: None,
            required: true,
            waivable: false,
            timeout_secs: Some(5),
            triggers: vec![],
            note: None,
        }
    }

    fn obs(ended: Ended) -> Observation {
        Observation {
            ended,
            stdout: String::new(),
            stderr: String::new(),
            report: None,
        }
    }

    #[test]
    fn exit_zero_passes_only_under_exit_zero_validity() {
        let v = classify(&check(Validity::ExitZero), &obs(Ended::Exited(0)));
        assert_eq!(v.outcome, Outcome::Passed);
        assert_eq!(v.exit_status, Some(0));
    }

    #[test]
    fn failing_command_is_failed_with_its_exit_status() {
        let v = classify(&check(Validity::ExitZero), &obs(Ended::Exited(101)));
        assert_eq!(v.outcome, Outcome::Failed);
        assert_eq!(v.exit_status, Some(101));
        assert_eq!(v.error_class.as_deref(), Some("nonzero_exit"));
    }

    #[test]
    fn missing_tool_and_timeout_are_never_passed() {
        let missing = classify(&check(Validity::ExitZero), &obs(Ended::NotFound));
        assert_eq!(missing.outcome, Outcome::NotRun);
        assert_eq!(missing.exit_status, None);
        assert!(missing.reason.unwrap().starts_with("tool_missing"));
        let timeout = classify(&check(Validity::ExitZero), &obs(Ended::TimedOut));
        assert_eq!(timeout.outcome, Outcome::Failed);
        assert_eq!(timeout.error_class.as_deref(), Some("timeout"));
        let signal = classify(&check(Validity::ExitZero), &obs(Ended::Signalled));
        assert_eq!(signal.outcome, Outcome::Failed);
    }

    #[test]
    fn junit_needs_a_fresh_report_with_selected_tests() {
        let c = check(Validity::JunitTestsSelected);
        let absent = classify(&c, &obs(Ended::Exited(0)));
        assert_eq!(absent.error_class.as_deref(), Some("invalid_report"));

        let mut zero = obs(Ended::Exited(0));
        zero.report =
            Some(r#"<testsuites name="nextest-run" tests="0" failures="0" errors="0">"#.into());
        let v = classify(&c, &zero);
        assert_eq!(
            (v.outcome, v.error_class.as_deref()),
            (Outcome::Failed, Some("validity"))
        );

        let mut malformed = obs(Ended::Exited(0));
        malformed.report = Some("<testsuite tests=\"3\">".into());
        assert_eq!(
            classify(&c, &malformed).error_class.as_deref(),
            Some("invalid_report")
        );

        let mut good = obs(Ended::Exited(0));
        good.report = Some(r#"<?xml version="1.0"?><testsuites name="nextest-run" tests="7" failures="0" errors="0" time="1.0">"#.into());
        let v = classify(&c, &good);
        assert_eq!(v.outcome, Outcome::Passed);
        assert_eq!(v.selection_counts.unwrap()["tests"], 7);
    }

    #[test]
    fn architecture_report_and_exit_status_must_agree() {
        let c = check(Validity::ArchitectureReportPassed);
        let not_run_report = r#"{"summary":{"outcome":"not_run","reason":"not_implemented"}}"#;

        let mut stub = obs(Ended::Exited(EXIT_NOT_RUN));
        stub.report = Some(not_run_report.into());
        let v = classify(&c, &stub);
        assert_eq!(v.outcome, Outcome::NotRun);
        assert_eq!(v.exit_status, None);

        let mut lying = obs(Ended::Exited(0));
        lying.report = Some(not_run_report.into());
        assert_eq!(
            classify(&c, &lying).error_class.as_deref(),
            Some("invalid_report")
        );

        let no_report = classify(&c, &obs(Ended::Exited(EXIT_NOT_RUN)));
        assert_eq!(no_report.error_class.as_deref(), Some("invalid_report"));

        let mut passed = obs(Ended::Exited(0));
        passed.report = Some(r#"{"summary":{"outcome":"passed","errors":0,"crates":1}}"#.into());
        assert_eq!(classify(&c, &passed).outcome, Outcome::Passed);

        let mut found = obs(Ended::Exited(1));
        found.report = Some(r#"{"summary":{"outcome":"failed","errors":2}}"#.into());
        assert_eq!(
            classify(&c, &found).error_class.as_deref(),
            Some("violation")
        );
    }

    #[test]
    fn dependency_network_failures_are_named() {
        let mut c = check(Validity::ExitZero);
        c.kind = "dependency".into();
        let mut o = obs(Ended::Exited(1));
        o.stderr = "error: failed to fetch advisory database".into();
        assert_eq!(classify(&c, &o).error_class.as_deref(), Some("network"));
    }

    #[test]
    fn doctest_counts_every_harness_run() {
        let c = check(Validity::DoctestHarnessRan);
        let mut o = obs(Ended::Exited(0));
        o.stdout = "running 2 tests\ntest result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.3s\n\nrunning 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.0s\n".into();
        let v = classify(&c, &o);
        assert_eq!(v.outcome, Outcome::Passed);
        let counts = v.selection_counts.unwrap();
        assert_eq!(
            (counts["passed"], counts["ignored"], counts["harness_runs"]),
            (2, 1, 2)
        );
        let silent = classify(&c, &obs(Ended::Exited(0)));
        assert_eq!(silent.error_class.as_deref(), Some("validity"));
    }

    #[test]
    fn gate_skips_unapproved_unselected_and_unavailable_checks() {
        let c = check(Validity::ExitZero);
        let mut unapproved = c.clone();
        unapproved.argv.clear();
        assert!(
            gate(&unapproved, true, None)
                .unwrap()
                .reason
                .unwrap()
                .starts_with("no_approved_command")
        );
        assert!(
            gate(&c, false, None)
                .unwrap()
                .reason
                .unwrap()
                .starts_with("not_selected")
        );
        let missing = Probe::Missing {
            detail: "no such command".into(),
        };
        assert!(
            gate(&c, true, Some(&missing))
                .unwrap()
                .reason
                .unwrap()
                .starts_with("tool_missing")
        );
        let old = Probe::Mismatch {
            installed: "1".into(),
            required: "2".into(),
        };
        assert!(
            gate(&c, true, Some(&old))
                .unwrap()
                .reason
                .unwrap()
                .starts_with("version_mismatch")
        );
        let ready = Probe::Ready {
            version: "2".into(),
        };
        assert_eq!(gate(&c, true, Some(&ready)), None);
    }

    #[test]
    fn exit_code_reports_failures_and_ci_environment_gaps_only() {
        let pass = Verdict::passed(0, None);
        let stub = Verdict::not_run("not_implemented: crate-graph checker (xtask exit 4)");
        let missing = Verdict::not_run("tool_missing: typos");
        let failed = Verdict::failed(Some(1), "nonzero_exit", "exited with 1");
        assert_eq!(exit_code(&[pass.clone(), stub.clone()], Label::Ci), 0);
        assert_eq!(exit_code(&[pass.clone(), missing.clone()], Label::Local), 0);
        assert_eq!(exit_code(&[pass.clone(), missing], Label::Ci), 1);
        assert_eq!(exit_code(&[pass, stub, failed], Label::Local), 1);
    }
}
