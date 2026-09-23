use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use library::Digest;
use serde_json::{Map, Value};

mod registered_package_embedded;

#[derive(Clone, Copy)]
struct Spec {
    cases: &'static str,
    registration: &'static str,
    sums: &'static str,
    count: usize,
    kind: &'static str,
}

fn spec(package: &str) -> Spec {
    match package {
        "sections" => Spec {
            cases: "d43d842b1f7b5eed0833eba2ad4131673299590f240d1f909d2b57760e05b0bf",
            registration: "c9061153c6bc5ae07bc0bff0ed5ec03c4a8ba80029180bae2c89691599525506",
            sums: "c727fe249b0e5fbfb3a3341a57972a7fbe9320cfe8b09b81ce78d493aacb3de9",
            count: 80,
            kind: "section",
        },
        "sites" => Spec {
            cases: "672232cf85a3712c3b7c621b40214df6c7ea23dfcbfe1ed5ba113d6dafd34118",
            registration: "cd0f68eef71a8a76c95a9324805795de6dea91f1ac6be4f1bd9fb167069681ce",
            sums: "af373c174af3f6ac7964474122c47f7004bdcf87354ac73d2cedbfa28e9139cb",
            count: 12,
            kind: "site",
        },
        "regions" => Spec {
            cases: "1812d1c3dc11a2c12116105590436768630886d8b00747c377efbe498f375b22",
            registration: "7506efe624e864b5c2fa17be47d72994f636cdcb47b2c8327ad7015839ca68f4",
            sums: "0a5f534edf4e6171961b5db2761f81237fa99e1f470b671247654ef5fed01812",
            count: 262,
            kind: "region_graph",
        },
        "uris" => Spec {
            cases: "5b9a07b9079c123dc58aa57cf87c1334ade164bc96fab37f53b5d7379cde90ad",
            registration: "6e5248bd43aafd2cd3fb5a673eddc0dc8d3bfa9346ca9b3923c4287b63efaac7",
            sums: "c2b2474dd57bb695d1d8b4c6f14ae7958f160cfb4c20fd98bbca5c4ae7e43126",
            count: 153,
            kind: "uri",
        },
        other => panic!("unknown registered transclusion package: {other}"),
    }
}

fn embedded_payloads(package: &str) -> BTreeMap<String, Vec<u8>> {
    spec(package);
    let prefix = format!("{package}/");
    registered_package_embedded::FILES
        .iter()
        .filter_map(|(path, bytes)| {
            path.strip_prefix(&prefix)
                .map(|relative| (relative.to_owned(), bytes.to_vec()))
        })
        .collect()
}

pub fn load(package: &str) -> Vec<Value> {
    let payloads = embedded_payloads(package);
    load_from_payloads(package, &payloads).unwrap_or_else(|error| panic!("{package}: {error}"))
}

pub fn load_from_payloads(
    package: &str,
    payloads: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Value>, String> {
    verify_payloads(package, payloads)?;
    let bytes = payload(payloads, "CASES.json")?;
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("{package}/CASES.json is not JSON: {error}"))?;
    validate_cases(package, &value)
}

pub fn verify_payloads(package: &str, payloads: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let expected = spec(package);
    let sums = payload(payloads, "SHA256SUMS")?;
    let registration = payload(payloads, "registration.toml")?;
    let cases = payload(payloads, "CASES.json")?;

    if digest(sums) != expected.sums {
        return Err("SHA256SUMS digest is not the registered digest".to_owned());
    }
    if digest(registration) != expected.registration {
        return Err("registration.toml digest is not the registered digest".to_owned());
    }
    if digest(cases) != expected.cases {
        return Err("CASES.json digest is not the registered digest".to_owned());
    }

    let listed = parse_sums(sums)?;
    if listed.len() != 7 {
        return Err(format!(
            "expected 7 registered payloads, found {}",
            listed.len()
        ));
    }

    let mut expected_files = listed.keys().cloned().collect::<BTreeSet<_>>();
    expected_files.insert("CASES.json".to_owned());
    expected_files.insert("registration.toml".to_owned());
    expected_files.insert("SHA256SUMS".to_owned());
    let actual_files = payloads.keys().cloned().collect::<BTreeSet<_>>();
    if actual_files != expected_files {
        return Err(format!(
            "payload closure mismatch: actual={actual_files:?}, expected={expected_files:?}"
        ));
    }

    for (relative, expected_digest) in listed {
        let bytes = payload(payloads, &relative)?;
        let actual = digest(bytes);
        if actual != expected_digest {
            return Err(format!("{relative} digest mismatch"));
        }
    }

    // The frozen registration.toml and SHA256SUMS digests are independent pins;
    // their declared relationship was measured at registration.
    Ok(())
}

fn payload<'a>(
    payloads: &'a BTreeMap<String, Vec<u8>>,
    relative: &str,
) -> Result<&'a [u8], String> {
    payloads
        .get(relative)
        .map(|bytes| bytes.as_slice())
        .ok_or_else(|| format!("missing payload: {relative}"))
}

fn digest(bytes: &[u8]) -> String {
    Digest::of(bytes).to_string()
}

fn parse_sums(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("SHA256SUMS UTF-8: {error}"))?;
    if !text.ends_with('\n') {
        return Err("SHA256SUMS must end with LF".to_owned());
    }

    let mut previous = None;
    let mut listed = BTreeMap::new();
    for line in text[..text.len() - 1].split('\n') {
        if line.contains('\r') {
            return Err("SHA256SUMS contains CR".to_owned());
        }
        let Some((digest, relative)) = line.split_once("  ") else {
            return Err(format!("malformed SHA256SUMS line: {line:?}"));
        };
        if !lower_sha256(digest) || !safe_relative_path(relative) {
            return Err(format!("invalid SHA256SUMS entry: {line:?}"));
        }
        if matches!(relative, "registration.toml" | "SHA256SUMS") {
            return Err(format!("metadata listed as payload: {relative}"));
        }
        if let Some(previous) = previous
            && previous >= relative
        {
            return Err("SHA256SUMS paths are not strictly sorted".to_owned());
        }
        previous = Some(relative);
        if listed
            .insert(relative.to_owned(), digest.to_owned())
            .is_some()
        {
            return Err(format!("duplicate SHA256SUMS path: {relative}"));
        }
    }
    Ok(listed)
}

fn lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(value).is_relative()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub fn validate_cases(package: &str, value: &Value) -> Result<Vec<Value>, String> {
    let cases = value
        .as_array()
        .ok_or_else(|| "CASES.json root must be an array".to_owned())?;
    let expected = spec(package);
    if cases.len() != expected.count {
        return Err(format!(
            "expected {} cases, found {}",
            expected.count,
            cases.len()
        ));
    }

    let mut ids = BTreeSet::new();
    for case in cases {
        let object = case
            .as_object()
            .ok_or_else(|| "case must be an object".to_owned())?;
        let id = string(object, "id", "case")?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate case id: {id}"));
        }
        if string(object, "kind", "case")? != expected.kind {
            return Err(format!("unexpected case kind for {id}"));
        }
        match package {
            "sections" => validate_section_case(object, id)?,
            "sites" => validate_site_case(object, id)?,
            "regions" => validate_region_case(object, id)?,
            "uris" => validate_uri_case(object, id)?,
            _ => unreachable!(),
        }
    }
    Ok(cases.clone())
}

fn validate_section_case(object: &Map<String, Value>, id: &str) -> Result<(), String> {
    exact_keys(
        object,
        &[
            "id",
            "kind",
            "body",
            "anchor",
            "expected",
            "expected_transclusions",
        ],
        id,
    )?;
    nodes(
        object["body"]
            .as_array()
            .ok_or_else(|| format!("{id}.body"))?,
        id,
    )?;
    if !object["expected"].is_null() {
        nodes(
            object["expected"]
                .as_array()
                .ok_or_else(|| format!("{id}.expected"))?,
            id,
        )?;
    }
    let descriptors = object["expected_transclusions"]
        .as_array()
        .ok_or_else(|| format!("{id}.expected_transclusions"))?;
    for descriptor in descriptors {
        let descriptor = descriptor
            .as_object()
            .ok_or_else(|| format!("{id}.expected_transclusions entry"))?;
        exact_keys(
            descriptor,
            &["kind", "id", "target", "anchor", "display", "line"],
            id,
        )?;
    }
    Ok(())
}

fn nodes(values: &[Value], id: &str) -> Result<(), String> {
    for node in values {
        let object = node.as_object().ok_or_else(|| format!("{id}: node"))?;
        let kind = string(object, "kind", id)?;
        match kind {
            "Heading" => {
                exact_keys(object, &["kind", "level", "slug", "children"], id)?;
                nodes(array(object, "children", id)?, id)?;
            }
            "Paragraph" | "Emphasis" | "Strong" | "Strikethrough" => {
                exact_keys(object, &["kind", "children"], id)?;
                nodes(array(object, "children", id)?, id)?;
            }
            "Text" | "Code" | "Html" => {
                exact_keys(object, &["kind", "text"], id)?;
            }
            "CodeBlock" => {
                exact_keys(object, &["kind", "lang", "text"], id)?;
            }
            "Link" => {
                exact_keys(object, &["kind", "href", "children"], id)?;
                nodes(array(object, "children", id)?, id)?;
            }
            "WikiLink" => {
                exact_keys(object, &["kind", "index", "children"], id)?;
                nodes(array(object, "children", id)?, id)?;
            }
            "Image" => {
                exact_keys(object, &["kind", "src", "alt"], id)?;
            }
            "List" => {
                exact_keys(object, &["kind", "start", "items"], id)?;
                for item in array(object, "items", id)? {
                    nodes(
                        item.as_array().ok_or_else(|| format!("{id}: list item"))?,
                        id,
                    )?;
                }
            }
            "BlockQuote" => {
                exact_keys(object, &["kind", "callout", "children"], id)?;
                nodes(array(object, "children", id)?, id)?;
            }
            "Table" => {
                exact_keys(object, &["kind", "align", "head", "rows"], id)?;
                for cell in array(object, "head", id)? {
                    nodes(
                        cell.as_array().ok_or_else(|| format!("{id}: table head"))?,
                        id,
                    )?;
                }
                for row in array(object, "rows", id)? {
                    for cell in row.as_array().ok_or_else(|| format!("{id}: table row"))? {
                        nodes(
                            cell.as_array().ok_or_else(|| format!("{id}: table cell"))?,
                            id,
                        )?;
                    }
                }
            }
            "Rule" => exact_keys(object, &["kind"], id)?,
            "SoftBreak" | "HardBreak" => exact_keys(object, &["kind"], id)?,
            "TaskMarker" => exact_keys(object, &["kind", "checked"], id)?,
            "Transclusion" => {
                exact_keys(
                    object,
                    &["kind", "id", "target", "anchor", "display", "line"],
                    id,
                )?;
            }
            other => return Err(format!("{id}: unknown node kind {other}")),
        }
    }
    Ok(())
}

fn validate_site_case(object: &Map<String, Value>, id: &str) -> Result<(), String> {
    let mut keys = vec![
        "id",
        "kind",
        "sources",
        "expected_documents",
        "expected_check",
        "expected_exit",
        "expected_pages",
    ];
    if object.contains_key("selector_queries") {
        keys.push("selector_queries");
    }
    if object.contains_key("expected_html_contains") {
        keys.push("expected_html_contains");
    }
    exact_keys(object, &keys, id)?;

    for source in array(object, "sources", id)? {
        let source = source.as_object().ok_or_else(|| format!("{id}: source"))?;
        exact_keys(source, &["path", "text"], id)?;
    }
    let documents = object
        .get("expected_documents")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{id}.expected_documents"))?;
    for document in documents.values() {
        let document = document
            .as_object()
            .ok_or_else(|| format!("{id}: document"))?;
        exact_keys(
            document,
            &[
                "transclusions",
                "navigation_links",
                "images",
                "heading_slugs",
            ],
            id,
        )?;
        for descriptor in array(document, "transclusions", id)? {
            exact_keys(
                descriptor
                    .as_object()
                    .ok_or_else(|| format!("{id}: transclusion"))?,
                &["id", "target", "anchor", "display", "line"],
                id,
            )?;
        }
        for link in array(document, "navigation_links", id)? {
            exact_keys(
                link.as_object()
                    .ok_or_else(|| format!("{id}: navigation link"))?,
                &["target", "anchor", "alias", "line"],
                id,
            )?;
        }
        for image in array(document, "images", id)? {
            exact_keys(
                image.as_object().ok_or_else(|| format!("{id}: image"))?,
                &["src", "alt"],
                id,
            )?;
        }
    }
    if let Some(queries) = object.get("selector_queries") {
        for query in queries
            .as_array()
            .ok_or_else(|| format!("{id}.selector_queries"))?
        {
            exact_keys(
                query
                    .as_object()
                    .ok_or_else(|| format!("{id}: selector query"))?,
                &[
                    "source",
                    "anchor",
                    "selected_heading_slugs",
                    "excluded_heading_slugs",
                    "wrapper",
                    "ordered_start",
                ],
                id,
            )?;
        }
    }
    Ok(())
}

fn validate_region_case(object: &Map<String, Value>, id: &str) -> Result<(), String> {
    exact_keys(object, &["id", "kind", "vertices", "edges", "expected"], id)?;
    for vertex in array(object, "vertices", id)? {
        exact_keys(
            vertex.as_object().ok_or_else(|| format!("{id}: vertex"))?,
            &["page", "anchor"],
            id,
        )?;
    }
    for edge in array(object, "edges", id)? {
        let edge = edge.as_object().ok_or_else(|| format!("{id}: edge"))?;
        exact_keys(edge, &["source", "transclusion_id", "target"], id)?;
        exact_keys(
            edge["source"]
                .as_object()
                .ok_or_else(|| format!("{id}: edge source"))?,
            &["page", "anchor"],
            id,
        )?;
        exact_keys(
            edge["target"]
                .as_object()
                .ok_or_else(|| format!("{id}: edge target"))?,
            &["page", "anchor"],
            id,
        )?;
    }
    let expected = object["expected"]
        .as_object()
        .ok_or_else(|| format!("{id}.expected"))?;
    exact_keys(expected, &["blocked", "cycles", "remaining_is_dag"], id)?;
    Ok(())
}

fn validate_uri_case(object: &Map<String, Value>, id: &str) -> Result<(), String> {
    exact_keys(
        object,
        &[
            "anchors",
            "destination",
            "expected",
            "host",
            "id",
            "kind",
            "origin",
            "reference_kind",
        ],
        id,
    )?;
    Ok(())
}

fn exact_keys(object: &Map<String, Value>, expected: &[&str], context: &str) -> Result<(), String> {
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "{context}: unexpected keys: actual={actual:?}, expected={expected:?}"
        ));
    }
    Ok(())
}

fn string<'a>(object: &'a Map<String, Value>, key: &str, context: &str) -> Result<&'a str, String> {
    object[key]
        .as_str()
        .ok_or_else(|| format!("{context}.{key} must be a string"))
}

fn array<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, String> {
    object[key]
        .as_array()
        .ok_or_else(|| format!("{context}.{key} must be an array"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_packages_have_exact_closure_and_fail_closed_payload_controls() {
        assert_eq!(registered_package_embedded::FILES.len(), 36);

        for (package, count, kind) in [
            ("regions", 262, "region_graph"),
            ("sections", 80, "section"),
            ("sites", 12, "site"),
            ("uris", 153, "uri"),
        ] {
            let payloads = embedded_payloads(package);
            assert_eq!(payloads.len(), 9, "{package}: file closure");

            let cases = load_from_payloads(package, &payloads).expect(package);
            assert_eq!(cases.len(), count, "{package}: case count");
            assert!(
                cases.iter().all(|case| case["kind"].as_str() == Some(kind)),
                "{package}: case kind"
            );

            let mut corrupted = payloads.clone();
            corrupted
                .get_mut("CASES.json")
                .expect("embedded cases")
                .push(b'\n');
            assert_eq!(
                verify_payloads(package, &corrupted).expect_err("corrupted payload"),
                "CASES.json digest is not the registered digest"
            );

            let mut missing = payloads.clone();
            missing.remove("CASES.json");
            assert_eq!(
                verify_payloads(package, &missing).expect_err("missing payload"),
                "missing payload: CASES.json"
            );

            let mut extra = payloads;
            extra.insert("extra".to_owned(), Vec::new());
            let error = verify_payloads(package, &extra).expect_err("extra payload");
            assert!(
                error.starts_with("payload closure mismatch: "),
                "unexpected extra-payload error: {error}"
            );
        }
    }
}
