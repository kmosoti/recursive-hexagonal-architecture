use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::Value;

const SCHEMAS: &[(&str, &str)] = &[
    ("task", include_str!("../../.rha/schemas/task.schema.json")),
    (
        "acceptance",
        include_str!("../../.rha/schemas/acceptance.schema.json"),
    ),
    (
        "policy",
        include_str!("../../.rha/schemas/policy.schema.json"),
    ),
    (
        "evidence",
        include_str!("../../.rha/schemas/evidence.schema.json"),
    ),
    ("h4", include_str!("../../.rha/schemas/h4.schema.json")),
    (
        "markdown_corpus",
        include_str!("../../.rha/schemas/markdown_corpus.schema.json"),
    ),
    (
        "h5_conformance",
        include_str!("../../.rha/schemas/h5_conformance.schema.json"),
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

fn validators() -> &'static Result<BTreeMap<&'static str, jsonschema::Validator>, String> {
    static VALIDATORS: OnceLock<Result<BTreeMap<&'static str, jsonschema::Validator>, String>> =
        OnceLock::new();

    VALIDATORS.get_or_init(|| {
        let mut validators = BTreeMap::new();
        for &(family, source) in SCHEMAS {
            let schema: Value =
                serde_json::from_str(source).map_err(|error| format!("{family}: {error}"))?;
            let validator = jsonschema::draft202012::options()
                .offline()
                .build(&schema)
                .map_err(|error| format!("{family}: {error}"))?;
            validators.insert(family, validator);
        }
        Ok(validators)
    })
}

#[must_use]
pub fn configuration_error() -> Option<&'static str> {
    validators().as_ref().err().map(String::as_str)
}

fn issue_code(schema_path: &str) -> &'static str {
    match schema_path.rsplit('/').next().unwrap_or_default() {
        "required" => "schema.missing_field",
        "type" => "schema.wrong_type",
        "additionalProperties" | "unevaluatedProperties" => "schema.unknown_field",
        _ => "schema.invalid_value",
    }
}

fn sort_issues(issues: &mut Vec<Issue>) {
    issues.sort_by(|left, right| {
        (&left.code, &left.path, &left.message).cmp(&(&right.code, &right.path, &right.message))
    });
    issues.dedup_by(|left, right| {
        left.code == right.code && left.path == right.path && left.message == right.message
    });
}

#[must_use]
pub fn validate(value: &Value, family: &str) -> Vec<Issue> {
    let all = match validators() {
        Ok(all) => all,
        Err(message) => {
            return vec![Issue {
                code: "schema.definition_error",
                path: String::new(),
                message: message.clone(),
            }];
        }
    };
    let Some(validator) = all.get(family) else {
        return vec![Issue {
            code: "schema.invalid_value",
            path: String::new(),
            message: format!("unsupported record schema family {family}"),
        }];
    };

    let mut issues = validator
        .iter_errors(value)
        .map(|error| Issue {
            code: issue_code(&error.schema_path().to_string()),
            path: error.instance_path().to_string(),
            message: error.to_string(),
        })
        .collect::<Vec<_>>();
    sort_issues(&mut issues);
    issues
}
