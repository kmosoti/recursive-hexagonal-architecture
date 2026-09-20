//! H4 execution and advisory evidence. The manifest is input, never output.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Output;

use serde_json::{Value, json};

use super::{Case, Expected, Level, MANIFEST_PATH, Manifest, fixture, grade};
use crate::cli::CorpusArgs;
use crate::error::{Context as _, Error, Result};
use crate::util::{UtcTime, command, command_stdout, sha256_file};

fn compiler_observations(output: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout).lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()).filter(|message| {
        message["reason"] == "compiler-message" && message["message"]["level"] == "error"
    }).map(|message| {
        let diagnostic = &message["message"];
        let names = diagnostic["spans"].as_array().into_iter().flatten()
            .filter(|span| span["is_primary"] == true)
            .flat_map(|span| span["text"].as_array().into_iter().flatten())
            .filter_map(|line| {
                let source = line["text"].as_str()?;
                let start = usize::try_from(line["highlight_start"].as_u64()?).ok()?.checked_sub(1)?;
                let end = usize::try_from(line["highlight_end"].as_u64()?).ok()?.checked_sub(1)?;
                qualified_name(source, start, end)
            }).collect::<Vec<_>>().join(" ");
        json!({"error_code": diagnostic["code"]["code"], "crate": message["target"]["name"].as_str().unwrap_or("").replace('_', "-"), "names": names, "diagnostic": diagnostic})
    }).collect()
}

fn qualified_name(source: &str, mut start: usize, mut end: usize) -> Option<String> {
    let chars: Vec<_> = source.chars().collect();
    if start >= end || end > chars.len() {
        return None;
    }
    let part = |ch: char| ch.is_alphanumeric() || ch == '_' || ch == ':';
    if !chars[start..end].iter().all(|ch| part(*ch)) {
        return None;
    }
    while start > 0 && part(chars[start - 1]) {
        start -= 1;
    }
    while end < chars.len() && part(chars[end]) {
        end += 1;
    }
    Some(chars[start..end].iter().collect())
}

fn expectation(expected: Expected) -> &'static str {
    match expected {
        Expected::Detect => "detect",
        Expected::NoAlarm => "no_alarm",
        Expected::ExpectedMiss => "expected_miss",
        Expected::Reference => "reference",
    }
}

fn execute(
    root: &Path,
    case: &Case,
    workspace: &Path,
    expected_tool: &Value,
) -> Result<(Vec<String>, Output, Value, grade::Grade)> {
    if let Some(detector) = &case.detector {
        if detector != "cargo check --offline" {
            return Err(Error::new(format!("unsupported detector {detector}")));
        }
        let args = vec![
            "check".to_owned(),
            "--offline".to_owned(),
            "--manifest-path".to_owned(),
            workspace.join("Cargo.toml").display().to_string(),
            "--message-format=json".to_owned(),
        ];
        let output = command("cargo")
            .args(&args)
            .current_dir(workspace)
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .output()
            .context(|| "running compiler detector".to_owned())?;
        let facts = compiler_observations(&output);
        let graded = grade::compiler(case, output.status.code(), &facts);
        let mut argv = vec!["cargo".to_owned()];
        argv.extend(args);
        return Ok((
            argv,
            output,
            json!({"compiler_observations": facts}),
            graded,
        ));
    }
    let args = vec![
        "xtask".to_owned(),
        "architecture".to_owned(),
        "--manifest-path".to_owned(),
        workspace.join("Cargo.toml").display().to_string(),
        "--rules".to_owned(),
        workspace.join("rha-crates.toml").display().to_string(),
        "--format".to_owned(),
        "json".to_owned(),
    ];
    let output = command("cargo")
        .args(&args)
        .current_dir(root)
        .output()
        .context(|| "executing architecture checker".to_owned())?;
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    let mut graded = grade::architecture(case, output.status.code(), &report);
    if let Err(error) = validate_report(case, workspace, expected_tool, &report) {
        graded.passed = false;
        graded.detected = false;
        graded.documented_miss = false;
        graded.reasons.push(error.to_string());
    }
    let mut argv = vec!["cargo".to_owned()];
    argv.extend(args);
    Ok((argv, output, report, graded))
}

/// Require the report to identify the invoked workspace, every declared
/// member, the dependency population and the actual checker checkout.
///
/// # Errors
/// Returns a provenance/coverage mismatch or a rules-file read failure.
pub fn validate_report(case: &Case, workspace: &Path, tool: &Value, report: &Value) -> Result<()> {
    if report["tool"] != *tool
        || report["subject"]["manifest_path"] != json!(workspace.join("Cargo.toml"))
        || report["subject"]["workspace_root"] != json!(workspace)
        || report["subject"]["metadata_mode"] != "no_deps"
        || report["subject"]["rules_path"] != json!(workspace.join("rha-crates.toml"))
        || report["subject"]["rules_digest"]
            != json!(format!(
                "sha256:{}",
                sha256_file(&workspace.join("rha-crates.toml"))?
            ))
    {
        return Err(Error::new(
            "report producer, subject, mode or rules digest differs from the invoked fixture",
        ));
    }
    let classification = report["classification"]
        .as_array()
        .ok_or_else(|| Error::new("missing classification"))?;
    if classification.len() != case.crates.len()
        || case.crates.iter().any(|krate| {
            classification
                .iter()
                .filter(|entry| {
                    entry["name"] == krate.name
                        && entry["manifest_path"]
                            == json!(workspace.join(&krate.name).join("Cargo.toml"))
                })
                .count()
                != 1
        })
    {
        return Err(Error::new(
            "report did not examine every declared workspace member exactly once",
        ));
    }
    for (key, kind) in [
        ("normal", super::DepKind::Normal),
        ("dev", super::DepKind::Dev),
        ("build", super::DepKind::Build),
    ] {
        let count = case
            .crates
            .iter()
            .flat_map(|krate| &krate.deps)
            .filter(|dep| dep.kind == kind)
            .count();
        if report["edges_examined"][key] != json!(count) {
            return Err(Error::new(format!(
                "report {key} edge population differs from the declared workspace"
            )));
        }
    }
    for collection in ["findings", "harness_edges"] {
        let findings = report[collection]
            .as_array()
            .ok_or_else(|| Error::new(format!("missing {collection} array")))?;
        for finding in findings {
            let source = finding["from"]
                .as_str()
                .ok_or_else(|| Error::new("finding has no source crate"))?;
            if !case.crates.iter().any(|krate| krate.name == source)
                || finding["manifest_path"] != json!(workspace.join(source).join("Cargo.toml"))
                || (collection == "harness_edges" && finding["severity"] != "note")
            {
                return Err(Error::new(
                    "finding location or listed-fact severity is invalid",
                ));
            }
            let rule = finding["rule"].as_str().unwrap_or("");
            let edge_rule = rule.starts_with("dir.")
                || rule.starts_with("effect.core_disallowed_")
                || matches!(
                    rule,
                    "forbidden.edge" | "adapter.foreign_core" | "adapter.port_owner_wrong_kind"
                );
            if (edge_rule || !finding["kind"].is_null())
                && !matches!(finding["kind"].as_str(), Some("normal" | "dev" | "build"))
            {
                return Err(Error::new(
                    "edge finding has a missing or invalid dependency kind",
                ));
            }
            if !finding["kind"].is_null() || finding["rule"] == "graph.cycle" {
                let witness = &finding["witness"];
                if !witness["edge"].as_str().is_some_and(|s| !s.is_empty())
                    || !witness["declared_in"]
                        .as_str()
                        .is_some_and(|s| !s.is_empty())
                    || !witness["optional"].is_boolean()
                {
                    return Err(Error::new(
                        "edge finding is missing its declaration witness",
                    ));
                }
            }
            if let Some(kind) = finding["kind"].as_str() {
                let declared = case
                    .crates
                    .iter()
                    .filter(|krate| krate.name == source)
                    .flat_map(|krate| &krate.deps)
                    .any(|dep| {
                        let (dep_kind, section) = match dep.kind {
                            super::DepKind::Normal => ("normal", "dependencies"),
                            super::DepKind::Dev => ("dev", "dev-dependencies"),
                            super::DepKind::Build => ("build", "build-dependencies"),
                        };
                        let declared_in = dep.target.as_ref().map_or_else(
                            || format!("[{section}]"),
                            |target| format!("[target.'{target}'.{section}]"),
                        );
                        finding["to"] == dep.name
                            && kind == dep_kind
                            && finding["witness"]["edge"]
                                == format!("{source} -> {} ({kind})", dep.name)
                            && finding["witness"]["declared_in"] == declared_in
                            && finding["witness"]["optional"] == dep.optional
                            && finding["witness"]["target"] == json!(dep.target)
                            && finding["witness"]["rename"] == json!(dep.rename)
                    });
                if !declared {
                    return Err(Error::new(
                        "finding edge/declaration witness differs from the actual fixture input",
                    ));
                }
            }
            if rule == "graph.cycle" {
                let path: Vec<_> = finding["witness"]["edge"]
                    .as_str()
                    .and_then(|s| s.strip_prefix("cycle: "))
                    .unwrap_or("")
                    .split(" -> ")
                    .collect();
                let members: BTreeSet<_> = path.iter().copied().collect();
                if path.len() < 3
                    || path.first() != path.last()
                    || finding["members"] != json!(members)
                    || finding["witness"]["declared_in"]
                        != "[dependencies] of each member in the cycle"
                    || path.windows(2).any(|pair| {
                        !case
                            .crates
                            .iter()
                            .filter(|krate| krate.name == pair[0])
                            .flat_map(|krate| &krate.deps)
                            .any(|dep| {
                                dep.name == pair[1]
                                    && dep.kind != super::DepKind::Dev
                                    && dep.source.as_deref().unwrap_or("path") == "path"
                                    && case.crates.iter().any(|krate| krate.name == pair[1])
                            })
                    })
                {
                    return Err(Error::new(
                        "cycle witness is not a closed path of declared member dependencies",
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Render every cell, downgrading all affected cells on any failed case.
/// Module-only cells have no crate result and make no validation claim.
#[must_use]
pub fn enforcement_map(manifest: &Manifest, cases: &[Value], evidence: &str) -> String {
    let mut text = format!(
        "# Enforcement map\n\nGenerated by `cargo xtask docs` from the [public manifest](../{MANIFEST_PATH}) and [{evidence}](../{evidence}). Crate scope only; module checks await CHG-007. A corpus result is advisory; held-out acceptance is Kennedy's (DP-1.1b), and Accepted maturity is unchanged (DP-1.6).\n\n| Cell | Claim | Crate evidence | Crate status | Module status | Documented holes |\n| --- | --- | --- | --- | --- | --- |\n"
    );
    let classification_failed = cases.iter().any(|case| {
        case["cells"]
            .as_array()
            .is_some_and(|cells| cells.contains(&json!("classification")))
            && case["grade"]["passed"] != true
    });
    for (id, description) in &manifest.cells {
        let relevant: Vec<_> = cases
            .iter()
            .filter(|c| {
                c["cells"]
                    .as_array()
                    .is_some_and(|cells| cells.contains(&json!(id)))
            })
            .collect();
        let failed = classification_failed || relevant.iter().any(|c| c["grade"]["passed"] != true);
        let ids = relevant
            .iter()
            .filter_map(|c| c["id"].as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let holes = relevant
            .iter()
            .filter_map(|c| c["hole"].as_str())
            .map(|s| s.replace('|', "\\|").replace('\n', " "))
            .collect::<Vec<_>>()
            .join("; ");
        let status = if failed {
            "review — downgraded"
        } else if relevant.is_empty() {
            "not_run — no crate cases"
        } else {
            "validated on registered crate cases only"
        };
        let _ = writeln!(
            text,
            "| `{id}` | {} | {ids} | {status} | not_run — CHG-007 | {holes} |",
            description.replace('|', "\\|")
        );
    }
    text
}

/// Totals include unmatched checker findings on every fixture; compiler
/// diagnostics are scored as a separate population.
#[must_use]
pub fn summarize(records: &[Value]) -> Value {
    let count = |expected: &str, field: Option<&str>, checker_only: bool| {
        records
            .iter()
            .filter(|c| {
                c["expected"] == expected
                    && (!checker_only || c["detector"] == "xtask architecture")
                    && field.is_none_or(|f| c["grade"][f] == true)
            })
            .count()
    };
    let detected = count("detect", Some("detected"), false);
    let violations = count("detect", None, false);
    let legitimate = count("no_alarm", None, true);
    let legitimate_alarms = records
        .iter()
        .filter(|c| {
            c["expected"] == "no_alarm"
                && c["grade"]["false_alarms"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty())
        })
        .count();
    let false_alarms: usize = records
        .iter()
        .filter(|c| c["detector"] == "xtask architecture")
        .map(|c| c["grade"]["false_alarms"].as_array().map_or(0, Vec::len))
        .sum();
    let failed = records.iter().any(|c| c["grade"]["passed"] != true);
    json!({"outcome": if failed { "failed" } else { "passed" }, "detection": {"detected": detected, "violations": violations}, "checker": {"detected": count("detect", Some("detected"), true), "cases": count("detect", None, true)}, "false_alarm": {"alarms": false_alarms, "legitimate": legitimate, "legitimate_cases_with_alarms": legitimate_alarms}, "expected_miss": {"cases": count("expected_miss", None, true), "documented": count("expected_miss", Some("documented_miss"), true), "surprises": count("expected_miss", Some("unexpected_detection"), true)}, "compiler": {"cases": records.iter().filter(|c| c["detector"] != "xtask architecture").count(), "detected": records.iter().filter(|c| c["detector"] != "xtask architecture" && c["grade"]["detected"] == true).count()}, "failed_cases": records.iter().filter(|c| c["grade"]["passed"] != true).map(|c| &c["id"]).collect::<Vec<_>>()})
}

/// Run all public committed crate cases and write one immutable record.
///
/// # Errors
/// Returns unsupported-level, invalid manifest or evidence I/O failures.
pub fn run(root: &Path, args: &CorpusArgs) -> Result<u8> {
    if args.level != "crate" {
        return Err(Error::new("module corpus not_implemented until CHG-007"));
    }
    let manifest_text = std::fs::read_to_string(root.join(MANIFEST_PATH))
        .context(|| "reading public corpus manifest".to_owned())?;
    let manifest =
        Manifest::parse(&manifest_text).context(|| "parsing public corpus manifest".to_owned())?;
    let defects = manifest.defects();
    if manifest.schema_version != 1 || !defects.is_empty() {
        return Err(Error::new(format!("invalid corpus manifest: {defects:?}")));
    }
    let cases: Vec<_> = manifest
        .cases
        .iter()
        .filter(|c| c.level == Level::Crate)
        .collect();
    if cases.is_empty() {
        return Err(Error::new("empty crate corpus"));
    }
    let now = UtcTime::now();
    let subject = crate::evidence::subject::capture(root, &root.join("target/rha/h4-subject"))?;
    let checker = std::env::current_exe().context(|| "locating checker executable".to_owned())?;
    let expected_tool = crate::graph::report::tool_identity(root);
    let template_path = root.join(crate::clippy_template::TEMPLATE_PATH);
    let expected_inputs = fixture::expected_tree(root, &manifest)?;
    let fixtures_root = root.join(fixture::COMMITTED_ROOT);
    let fixture_drift = fixture::drift(&fixtures_root, &expected_inputs)?;
    let run_id = format!(
        "{}-{}-{}{}",
        now.compact(),
        &subject.revision[..12],
        std::process::id(),
        if subject.dirty { "-dirty" } else { "" }
    );
    let mut records = Vec::new();
    for case in cases {
        let observed = (|| -> Result<Value> {
            if !fixture_drift.is_empty() {
                return Err(Error::new(format!(
                    "committed fixture drift: {fixture_drift:?}; run corpus generate --check"
                )));
            }
            let workspace = fixtures_root.join(fixture::case_path(case));
            let (argv, output, report, grade) = execute(root, case, &workspace, &expected_tool)?;
            Ok(
                json!({"fixture_path": workspace.strip_prefix(root).unwrap_or(&workspace), "argv": argv, "exit_status": output.status.code(), "stdout": String::from_utf8_lossy(&output.stdout), "stderr": String::from_utf8_lossy(&output.stderr), "report": report, "grade": grade}),
            )
        })();
        let mut record = match observed {
            Ok(value) => value,
            Err(error) => {
                json!({"grade": grade::Grade { reasons: vec![error.to_string()], ..Default::default() }, "execution_error": error.to_string()})
            }
        };
        record["id"] = json!(case.id);
        record["cells"] = json!(case.cells);
        record["expected"] = json!(expectation(case.expected));
        record["detector"] = json!(case.detector.as_deref().unwrap_or("xtask architecture"));
        record["witness"] = json!(case.witness);
        record["hole"] = json!(case.hole);
        record["detected_when"] = json!(case.detected_when);
        println!(
            "{}: {}{}",
            case.id,
            if record["grade"]["passed"] == true {
                "passed"
            } else {
                "failed"
            },
            if record["grade"]["documented_miss"] == true {
                " (documented hole)"
            } else {
                ""
            }
        );
        records.push(record);
    }
    let summary = summarize(&records);
    let failed = summary["outcome"] != "passed";
    let evidence_dir = root.join(&args.evidence);
    std::fs::create_dir_all(&evidence_dir)
        .context(|| "creating H4 evidence directory".to_owned())?;
    let path = evidence_dir.join(format!("{run_id}.json"));
    let relative_path = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .display()
        .to_string();
    let mut downgraded: BTreeSet<_> = records
        .iter()
        .filter(|c| c["grade"]["passed"] != true)
        .flat_map(|c| c["cells"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    if downgraded.contains("classification") {
        downgraded.extend(manifest.cells.keys().map(String::as_str));
    }
    let record = json!({
        "schema_version": 2, "kind": "h4", "level": "crate", "evidence_class": "local", "advisory": true,
        "created_at": now.rfc3339(), "artifact_identity": subject,
        "producer": {"name": "xtask corpus run", "version": env!("CARGO_PKG_VERSION"), "executable_sha256": sha256_file(&checker)?, "checker": expected_tool},
        "pre_registration": {"change": "CHG-002", "identity": manifest.pre_registration, "revision": command_stdout(root, &["git", "rev-parse", "15d916a^{commit}"])?},
        "manifest": {"path": MANIFEST_PATH, "sha256": sha256_file(&root.join(MANIFEST_PATH))?, "correction_commit": command_stdout(root, &["git", "log", "-1", "--format=%H", "HEAD", "--", MANIFEST_PATH])?, "approved_amendment": "Kennedy: Approved and merged, PR 6, 2026-09-20; EM-M03 cells [law3-d1, law6-b3]"},
        "fixtures": {"root": fixture::COMMITTED_ROOT, "drift": fixture_drift, "inputs_sha256": expected_inputs.iter().map(|(path, bytes)| (path.display().to_string(), crate::util::sha256_hex(bytes))).collect::<std::collections::BTreeMap<_, _>>()},
        "template_sha256": sha256_file(&template_path)?, "grading": {"decision": "DP-1.1c", "detection_requires": manifest.grading.detection_requires, "extra_findings": manifest.grading.extra_findings, "no_alarm_scope": manifest.grading.no_alarm_scope, "expected_miss_surprise": manifest.grading.expected_miss_surprise},
        "environment": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH, "cargo": command_stdout(root, &["cargo", "--version"])?, "rustc": command_stdout(root, &["rustc", "--version"])?},
        "summary": summary, "cases": records, "downgraded_cells": downgraded,
        "held_out": {"outcome": "not_run", "reason": "Kennedy runs the private cases at acceptance (DP-1.1b); Executor never reads them"},
    });
    let bytes =
        serde_json::to_vec_pretty(&record).context(|| "serializing H4 evidence".to_owned())?;
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .context(|| format!("creating immutable evidence {}", path.display()))?;
    file.write_all(&bytes)
        .context(|| "writing H4 evidence".to_owned())?;
    println!(
        "detection {}/{}, false_alarm {}/{} (legitimate cases with alarms: {}); evidence {relative_path}",
        summary["detection"]["detected"],
        summary["detection"]["violations"],
        summary["false_alarm"]["alarms"],
        summary["false_alarm"]["legitimate"],
        summary["false_alarm"]["legitimate_cases_with_alarms"]
    );
    Ok(u8::from(failed))
}

#[cfg(test)]
mod tests {
    use super::qualified_name;

    #[test]
    fn diagnostic_path_is_taken_from_the_highlighted_token_only() {
        let line = "pub fn take(_: core_a::private_mod::Item) {}";
        assert_eq!(
            qualified_name(line, 23, 34).as_deref(),
            Some("core_a::private_mod::Item")
        );
        assert_eq!(
            qualified_name("wrong::Item /* core_a::private_mod::Item */", 0, 5).as_deref(),
            Some("wrong::Item")
        );
        assert_eq!(qualified_name("bad span", 0, 8), None);
        assert_eq!(qualified_name("x", 5, 8), None);
    }
}
