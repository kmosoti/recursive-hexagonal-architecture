//! Strict grading independent of the fixture generator and checker.

use serde::Serialize;
use serde_json::Value;

use super::{Case, Expected};

#[derive(Debug, Default, Serialize)]
pub struct Grade {
    pub passed: bool,
    pub detected: bool,
    pub documented_miss: bool,
    pub unexpected_detection: bool,
    pub false_alarms: Vec<Value>,
    pub reasons: Vec<String>,
}

/// Every witness key must exist and equal its corresponding observation.
/// No prose matching or reconstruction from the expected answer is used.
#[must_use]
pub fn matches(case: &Case, finding: &Value) -> bool {
    !case.witness.is_empty()
        && case.witness.iter().all(|(key, expected)| {
            serde_json::to_value(expected).is_ok_and(|value| finding.get(key) == Some(&value))
        })
}

/// Grade an architecture report, including its exit status and listed facts.
#[must_use]
pub fn architecture(case: &Case, exit: Option<i32>, report: &Value) -> Grade {
    let mut grade = Grade::default();
    let Some(findings) = report["findings"].as_array() else {
        grade.reasons.push(
            "checker produced no findings array (failed execution or malformed report)".to_owned(),
        );
        return grade;
    };
    if report["schema_version"] != 1
        || !report["summary"]["error_class"].is_null()
        || !matches!(exit, Some(0 | 1))
    {
        grade
            .reasons
            .push("checker configuration, tool, schema or exit failure".to_owned());
        return grade;
    }
    let errors = findings.iter().filter(|f| f["severity"] == "error").count();
    let warnings = findings
        .iter()
        .filter(|f| f["severity"] == "warning")
        .count();
    if findings.iter().any(|f| {
        !f["rule"].is_string()
            || !f["from"].is_string()
            || !matches!(f["severity"].as_str(), Some("error" | "warning" | "note"))
    }) || report["summary"]["errors"].as_u64() != u64::try_from(errors).ok()
        || report["summary"]["warnings"].as_u64() != u64::try_from(warnings).ok()
        || exit != Some(i32::from(errors > 0))
        || report["summary"]["outcome"] != if errors > 0 { "failed" } else { "passed" }
    {
        grade
            .reasons
            .push("checker report and exit status disagree".to_owned());
        return grade;
    }
    match case.expected {
        Expected::Detect => {
            let detection = |f: &Value| matches(case, f) && f["severity"] == "error";
            grade.detected = findings.iter().any(detection);
            grade.false_alarms = findings.iter().filter(|f| !detection(f)).cloned().collect();
            if !grade.detected {
                grade
                    .reasons
                    .push("registered witness was not detected".to_owned());
            }
        }
        Expected::NoAlarm => {
            grade.false_alarms.clone_from(findings);
            if !case.witness.is_empty() {
                let listed = case.witness.get("listed_as").and_then(toml::Value::as_str);
                let found = listed
                    .and_then(|key| report[key].as_array())
                    .is_some_and(|facts| {
                        facts.iter().filter(|fact| fact.is_object()).any(|fact| {
                            let mut fact = fact.clone();
                            fact["listed_as"] =
                                Value::String(listed.unwrap_or_default().to_owned());
                            fact["severity"] == "note" && matches(case, &fact)
                        })
                    });
                if !found {
                    grade.reasons.push(
                        "required listed fact is absent or does not match every key".to_owned(),
                    );
                }
            }
        }
        Expected::ExpectedMiss => {
            grade.unexpected_detection = if case.witness.is_empty() {
                !findings.is_empty()
            } else {
                findings.iter().any(|f| matches(case, f))
            };
            grade.false_alarms = findings
                .iter()
                .filter(|f| !matches(case, f))
                .cloned()
                .collect();
            grade.documented_miss = findings.is_empty();
            if grade.unexpected_detection {
                grade.reasons.push("expected miss was detected: acceptance authority must adjudicate; no re-registration".to_owned());
            }
        }
        Expected::Reference => grade
            .reasons
            .push("reference cases are not supported by the crate harness".to_owned()),
    }
    if !grade.false_alarms.is_empty() {
        grade
            .reasons
            .push("unmatched findings are false alarms under DP-1.1c".to_owned());
    }
    grade.passed = grade.reasons.is_empty();
    grade
}

/// Compiler observations contain one fact per error diagnostic, with the
/// source names taken from the diagnostic's highlighted source lines.
#[must_use]
pub fn compiler(case: &Case, exit: Option<i32>, observations: &[Value]) -> Grade {
    let mut grade = Grade {
        detected: exit == Some(101) && observations.iter().any(|f| matches(case, f)),
        false_alarms: observations
            .iter()
            .filter(|f| !matches(case, f))
            .cloned()
            .collect(),
        ..Default::default()
    };
    if !grade.detected {
        grade.reasons.push(
            "compiler did not report the complete registered witness with exit 101".to_owned(),
        );
    }
    if !grade.false_alarms.is_empty() {
        grade
            .reasons
            .push("compiler emitted unrelated error diagnostics".to_owned());
    }
    grade.passed = grade.reasons.is_empty();
    grade
}
