use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::util::sha256_hex;

use super::extract::{Edge, Extracted};

#[derive(Debug)]
pub struct Checked {
    pub findings: Vec<Value>,
    pub test_edges: Vec<Value>,
    pub module_edges: Vec<Value>,
    pub limitations: Vec<Value>,
    pub rules_path: String,
    pub rules_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesFile {
    components: BTreeMap<String, String>,
    allow: BTreeMap<String, Vec<String>>,
    deny: DenyRules,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DenyRules {
    cycles: bool,
    child_to_parent_private: bool,
    foreign_internal: bool,
}

#[derive(Debug, Clone)]
struct Component {
    path: Vec<String>,
}

type EdgeSet = BTreeSet<Edge>;
type Pair = (String, String);

pub fn check(
    crate_root: &Path,
    crate_name: &str,
    rules_path: &Path,
    graph: &Extracted,
) -> Result<Checked, String> {
    let crate_root = fs::canonicalize(crate_root)
        .map_err(|error| format!("failed to canonicalize {}: {error}", crate_root.display()))?;
    let rules_path = fs::canonicalize(rules_path)
        .map_err(|error| format!("failed to canonicalize {}: {error}", rules_path.display()))?;
    if !rules_path.starts_with(&crate_root) {
        return Err(format!(
            "module rules file {} is outside crate root {}",
            rules_path.display(),
            crate_root.display()
        ));
    }

    let bytes = fs::read(&rules_path)
        .map_err(|error| format!("failed to read {}: {error}", rules_path.display()))?;
    let rules_digest = sha256_hex(&bytes);
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("module rules file is not UTF-8: {error}"))?;
    let rules: RulesFile = toml::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", rules_path.display()))?;

    let normalized_crate = crate_name.replace('-', "_");
    if normalized_crate.trim().is_empty() {
        return Err("crate name must not be blank".to_owned());
    }

    let mut components = BTreeMap::new();
    let mut paths = BTreeSet::new();
    if rules.components.is_empty() {
        return Err("module rules must declare at least one component".to_owned());
    }
    for (alias, path) in &rules.components {
        if alias.trim().is_empty() {
            return Err("module component aliases must not be blank".to_owned());
        }
        let parts = split_path(path);
        if parts.is_empty() || parts.iter().any(|part| part.trim().is_empty()) {
            return Err(format!("component {alias} has a blank path segment"));
        }
        if parts.first() != Some(&normalized_crate) {
            return Err(format!(
                "component {alias} must begin with normalized crate prefix {normalized_crate}"
            ));
        }
        let key = parts.join("::");
        if !paths.insert(key.clone()) {
            return Err(format!("duplicate component path {key}"));
        }
        if !graph.modules.contains_key(&key) {
            return Err(format!(
                "component {alias} refers to undiscovered module {key}"
            ));
        }
        components.insert(alias.clone(), Component { path: parts });
    }

    for (source, targets) in &rules.allow {
        if !components.contains_key(source) {
            return Err(format!("allow entry names unknown component {source}"));
        }
        for target in targets {
            if !components.contains_key(target) {
                return Err(format!(
                    "allow entry for {source} names unknown component {target}"
                ));
            }
        }
    }

    let mut module_edges = graph.edges.iter().map(raw_edge_value).collect::<Vec<_>>();
    module_edges.sort_by_key(|edge| edge.to_string());

    let mut test_edges = Vec::new();
    for edge in graph.edges.iter().filter(|edge| edge.test_only) {
        let source_parts = normalized_parts(&edge.source, crate_name, &normalized_crate);
        let target_parts = normalized_parts(&edge.target, crate_name, &normalized_crate);
        let target = component_owner(&target_parts, &components)
            .map(|(alias, _)| alias)
            .unwrap_or_else(|| canonical_path(&target_parts, &normalized_crate));
        test_edges.push(json!({
            "rule": "modules.test_edge",
            "severity": "note",
            "from": without_crate_prefix(&source_parts, &normalized_crate),
            "to": target,
        }));
    }
    test_edges.sort_by_key(Value::to_string);

    let mut limitations = graph
        .limitations
        .iter()
        .map(|limitation| {
            json!({
                "source": limitation.source,
                "code": limitation.code,
                "detail": limitation.detail,
            })
        })
        .collect::<Vec<_>>();
    add_disabled_limitation(
        &mut limitations,
        &normalized_crate,
        !rules.deny.cycles,
        "modules.cycle",
    );
    add_disabled_limitation(
        &mut limitations,
        &normalized_crate,
        !rules.deny.child_to_parent_private,
        "modules.child_to_parent_private",
    );
    add_disabled_limitation(
        &mut limitations,
        &normalized_crate,
        !rules.deny.foreign_internal,
        "modules.foreign_internal",
    );

    let mut d2: BTreeMap<(String, String, usize), EdgeSet> = BTreeMap::new();
    let mut d3: BTreeMap<(String, String), EdgeSet> = BTreeMap::new();
    let mut sibling_edges: BTreeMap<Pair, EdgeSet> = BTreeMap::new();
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = components
        .keys()
        .map(|alias| (alias.clone(), BTreeSet::new()))
        .collect();

    for edge in graph.edges.iter().filter(|edge| !edge.test_only) {
        let source_parts = normalized_parts(&edge.source, crate_name, &normalized_crate);
        let target_parts = normalized_parts(&edge.target, crate_name, &normalized_crate);
        let source_owner = component_owner(&source_parts, &components);
        let target_owner = component_owner(&target_parts, &components);
        let canonical_target = canonical_path(&target_parts, &normalized_crate);

        let Some((source_alias, source_component)) = source_owner else {
            if let Some(target_alias) = first_component_below(
                &target_parts,
                std::slice::from_ref(&normalized_crate),
                &components,
            ) {
                let facade = &components[&target_alias].path;
                if target_owner.is_some()
                    && longest_module_prefix(&target_parts, graph)
                        .is_some_and(|(depth, _)| depth > facade.len())
                    && rules.deny.foreign_internal
                {
                    d3.entry((
                        source_label(&source_parts, &normalized_crate),
                        canonical_target,
                    ))
                    .or_default()
                    .insert(edge.clone());
                }
            }
            continue;
        };

        if target_owner
            .as_ref()
            .is_some_and(|(target_alias, _)| target_alias == &source_alias)
        {
            continue;
        }

        let parent = parent_scope(&source_component.path, &components, &normalized_crate);
        if !is_prefix(&parent, &target_parts) {
            if rules.deny.child_to_parent_private {
                d2.entry((source_alias, canonical_target, source_component.path.len()))
                    .or_default()
                    .insert(edge.clone());
            }
            continue;
        }

        let Some(target_at_level) = first_component_below(&target_parts, &parent, &components)
        else {
            if rules.deny.child_to_parent_private {
                d2.entry((source_alias, canonical_target, source_component.path.len()))
                    .or_default()
                    .insert(edge.clone());
            }
            continue;
        };

        let target_facade = target_owner
            .as_ref()
            .map(|(_, component)| component.path.as_slice());
        let d3_facade = if target_at_level == source_alias {
            target_facade
        } else {
            Some(components[&target_at_level].path.as_slice())
        };
        if target_at_level != source_alias {
            adjacency
                .entry(source_alias.clone())
                .or_default()
                .insert(target_at_level.clone());
            sibling_edges
                .entry((source_alias.clone(), target_at_level.clone()))
                .or_default()
                .insert(edge.clone());
        }

        if rules.deny.foreign_internal
            && target_owner.is_some()
            && d3_facade.is_some_and(|facade| {
                longest_module_prefix(&target_parts, graph)
                    .is_some_and(|(depth, _)| depth > facade.len())
            })
        {
            d3.entry((source_alias, canonical_target))
                .or_default()
                .insert(edge.clone());
        }
    }

    let cyclic_components = if rules.deny.cycles {
        cyclic_components(&components, &adjacency)
    } else {
        Vec::new()
    };

    let mut findings = Vec::<(String, Value)>::new();
    for ((from, to, depth), edges) in d2 {
        let extraction = evidence_extraction(&edges);
        let edge = representative_edge(&edges, extraction == "heuristic");
        findings.push((
            format!("modules.child_to_parent_private\0{from}\0{to}\0{depth}"),
            json!({
                "rule": "modules.child_to_parent_private",
                "severity": "error",
                "from": from,
                "to": to,
                "depth": depth,
                "extraction": extraction,
                "message": format!("component {from} reaches outside its parent scope at {to}"),
                "manifest_path": crate_root.join("Cargo.toml").display().to_string(),
                "witness": witness(edge, graph, crate_name, &normalized_crate, &rules_path),
            }),
        ));
    }

    for ((from, to), edges) in d3 {
        let extraction = evidence_extraction(&edges);
        let edge = representative_edge(&edges, extraction == "heuristic");
        findings.push((
            format!("modules.foreign_internal\0{from}\0{to}"),
            json!({
                "rule": "modules.foreign_internal",
                "severity": "error",
                "from": from,
                "to": to,
                "extraction": extraction,
                "message": format!("component {from} reaches below a component facade at {to}"),
                "manifest_path": crate_root.join("Cargo.toml").display().to_string(),
                "witness": witness(edge, graph, crate_name, &normalized_crate, &rules_path),
            }),
        ));
    }

    for ((from, to), edges) in &sibling_edges {
        let subsumed = cyclic_components
            .iter()
            .any(|members| members.contains(from) && members.contains(to));
        if rules.deny.cycles && subsumed {
            continue;
        }
        if rules
            .allow
            .get(from)
            .is_some_and(|allowed| allowed.iter().any(|candidate| candidate == to))
        {
            continue;
        }
        let extraction = evidence_extraction(edges);
        let edge = representative_edge(edges, extraction == "heuristic");
        findings.push((
            format!("modules.undeclared_dependency\0{from}\0{to}"),
            json!({
                "rule": "modules.undeclared_dependency",
                "severity": "error",
                "from": from,
                "to": to,
                "extraction": extraction,
                "message": format!("component {from} depends on undeclared component {to}"),
                "manifest_path": crate_root.join("Cargo.toml").display().to_string(),
                "witness": witness(edge, graph, crate_name, &normalized_crate, &rules_path),
            }),
        ));
    }

    for members in cyclic_components {
        let start = members[0].clone();
        let Some(path) = shortest_return_path(&start, &members, &adjacency) else {
            continue;
        };
        let mut path_edges = Vec::new();
        let mut heuristic = false;
        for pair in path.windows(2) {
            let edges = sibling_edges
                .get(&(pair[0].clone(), pair[1].clone()))
                .expect("cycle adjacency has sibling edge evidence");
            heuristic |= edges.iter().any(|edge| edge.extraction == "heuristic");
            path_edges.push(representative_edge(edges, true).clone());
        }
        let extraction = if heuristic { "heuristic" } else { "syntax" };
        let first_edge = path_edges
            .first()
            .expect("cycle path has sibling edge evidence");

        let mut undeclared_edges = Vec::new();
        for pair in all_pairs(&members, &adjacency) {
            if !members.contains(&pair.0) || !members.contains(&pair.1) {
                continue;
            }
            let edges = sibling_edges
                .get(&pair)
                .expect("cycle adjacency has sibling edge evidence");
            if !rules
                .allow
                .get(&pair.0)
                .is_some_and(|allowed| allowed.iter().any(|candidate| candidate == &pair.1))
            {
                undeclared_edges.push(json!({
                    "from": pair.0,
                    "to": pair.1,
                    "extraction": evidence_extraction(edges),
                }));
            }
        }
        undeclared_edges.sort_by_key(Value::to_string);
        let subsumed_rules = if undeclared_edges.is_empty() {
            Vec::<&str>::new()
        } else {
            vec!["modules.undeclared_dependency"]
        };

        findings.push((
            format!("modules.cycle\0{}", members.join("\0")),
            json!({
                "rule": "modules.cycle",
                "severity": "error",
                "from": start,
                "members": members,
                "path": path,
                "extraction": extraction,
                "undeclared_edges": undeclared_edges,
                "subsumed_rules": subsumed_rules,
                "message": "declared components form a dependency cycle",
                "manifest_path": crate_root.join("Cargo.toml").display().to_string(),
                "witness": witness(
                    first_edge,
                    graph,
                    crate_name,
                    &normalized_crate,
                    &rules_path,
                ),
            }),
        ));
    }

    findings.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Checked {
        findings: findings.into_iter().map(|(_, finding)| finding).collect(),
        test_edges,
        module_edges,
        limitations,
        rules_path: rules_path.display().to_string(),
        rules_digest,
    })
}

fn split_path(path: &str) -> Vec<String> {
    path.split("::").map(ToOwned::to_owned).collect()
}

fn normalized_parts(path: &str, crate_name: &str, normalized_crate: &str) -> Vec<String> {
    let mut parts = split_path(path);
    if parts
        .first()
        .is_some_and(|part| part == crate_name || part == "crate")
    {
        parts[0] = normalized_crate.to_owned();
    }
    parts
}

fn canonical_path(parts: &[String], normalized_crate: &str) -> String {
    let mut parts = parts.to_vec();
    if parts.first() == Some(&normalized_crate.to_owned()) {
        parts[0] = "crate".to_owned();
    }
    parts.join("::")
}

fn without_crate_prefix(parts: &[String], normalized_crate: &str) -> String {
    if parts.first() == Some(&normalized_crate.to_owned()) {
        parts[1..].join("::")
    } else {
        parts.join("::")
    }
}

fn source_label(parts: &[String], normalized_crate: &str) -> String {
    let label = without_crate_prefix(parts, normalized_crate);
    if label.is_empty() {
        "crate".to_owned()
    } else {
        label
    }
}

fn is_prefix(prefix: &[String], value: &[String]) -> bool {
    value.starts_with(prefix)
}

fn component_owner<'a>(
    module: &[String],
    components: &'a BTreeMap<String, Component>,
) -> Option<(String, &'a Component)> {
    components
        .iter()
        .filter(|(_, component)| is_prefix(&component.path, module))
        .max_by_key(|(_, component)| component.path.len())
        .map(|(alias, component)| (alias.clone(), component))
}

fn parent_scope(
    component_path: &[String],
    components: &BTreeMap<String, Component>,
    normalized_crate: &str,
) -> Vec<String> {
    components
        .values()
        .filter(|component| {
            component.path.len() < component_path.len()
                && is_prefix(&component.path, component_path)
        })
        .max_by_key(|component| component.path.len())
        .map(|component| component.path.clone())
        .unwrap_or_else(|| vec![normalized_crate.to_owned()])
}

fn first_component_below(
    target: &[String],
    parent: &[String],
    components: &BTreeMap<String, Component>,
) -> Option<String> {
    components
        .iter()
        .filter(|(_, component)| {
            component.path.len() > parent.len()
                && is_prefix(parent, &component.path)
                && is_prefix(&component.path, target)
        })
        .min_by_key(|(alias, component)| (component.path.len(), (*alias).clone()))
        .map(|(alias, _)| alias.clone())
}

fn longest_module_prefix<'a>(
    target: &[String],
    graph: &'a Extracted,
) -> Option<(usize, &'a PathBuf)> {
    for length in (1..=target.len()).rev() {
        let key = target[..length].join("::");
        if let Some(path) = graph.modules.get(&key) {
            return Some((length, path));
        }
    }
    None
}

fn raw_edge_value(edge: &Edge) -> Value {
    json!({
        "source": edge.source,
        "target": edge.target,
        "test_only": edge.test_only,
        "extraction": edge.extraction,
    })
}

fn witness(
    edge: &Edge,
    graph: &Extracted,
    crate_name: &str,
    normalized_crate: &str,
    rules_path: &Path,
) -> Value {
    let source_parts = normalized_parts(&edge.source, crate_name, normalized_crate);
    let target_parts = normalized_parts(&edge.target, crate_name, normalized_crate);
    let source_key = source_parts.join("::");
    let target_file =
        longest_module_prefix(&target_parts, graph).map(|(_, path)| path.display().to_string());
    json!({
        "source": edge.source,
        "target": edge.target,
        "source_file": graph.modules.get(&source_key).map(|path| path.display().to_string()),
        "target_file": target_file,
        "rules_file": rules_path.display().to_string(),
    })
}

fn evidence_extraction(edges: &EdgeSet) -> &'static str {
    if edges.iter().any(|edge| edge.extraction == "heuristic") {
        "heuristic"
    } else {
        "syntax"
    }
}

fn representative_edge(edges: &EdgeSet, prefer_heuristic: bool) -> &Edge {
    if prefer_heuristic && let Some(edge) = edges.iter().find(|edge| edge.extraction == "heuristic")
    {
        return edge;
    }
    edges.iter().next().expect("an evidence set is non-empty")
}

fn reachable(start: &str, adjacency: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    seen.insert(start.to_owned());
    queue.push_back(start.to_owned());
    while let Some(node) = queue.pop_front() {
        if let Some(targets) = adjacency.get(&node) {
            for target in targets {
                if seen.insert(target.clone()) {
                    queue.push_back(target.clone());
                }
            }
        }
    }
    seen
}

fn cyclic_components(
    components: &BTreeMap<String, Component>,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<Vec<String>> {
    let aliases = components.keys().cloned().collect::<Vec<_>>();
    let mut sets = BTreeSet::<Vec<String>>::new();
    for start in &aliases {
        let forward = reachable(start, adjacency);
        let members = aliases
            .iter()
            .filter(|candidate| {
                forward.contains(*candidate) && reachable(candidate, adjacency).contains(start)
            })
            .cloned()
            .collect::<Vec<_>>();
        let self_loop = adjacency
            .get(start)
            .is_some_and(|targets| targets.contains(start));
        if members.len() > 1 || self_loop {
            sets.insert(members);
        }
    }
    sets.into_iter().collect()
}

fn shortest_return_path(
    start: &str,
    members: &[String],
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> Option<Vec<String>> {
    let member_set = members.iter().cloned().collect::<BTreeSet<_>>();
    let mut queue = VecDeque::from([vec![start.to_owned()]]);
    while let Some(path) = queue.pop_front() {
        let current = path.last()?;
        for target in adjacency.get(current).into_iter().flatten() {
            if !member_set.contains(target) {
                continue;
            }
            if target == start {
                if path.len() > 1 {
                    let mut closed = path;
                    closed.push(start.to_owned());
                    return Some(closed);
                }
                continue;
            }
            if !path.contains(target) {
                let mut next = path.clone();
                next.push(target.clone());
                queue.push_back(next);
            }
        }
    }
    None
}

fn all_pairs(members: &[String], adjacency: &BTreeMap<String, BTreeSet<String>>) -> Vec<Pair> {
    let mut pairs = BTreeSet::new();
    for source in members {
        if let Some(targets) = adjacency.get(source) {
            for target in targets {
                pairs.insert((source.clone(), target.clone()));
            }
        }
    }
    pairs.into_iter().collect()
}

fn add_disabled_limitation(limitations: &mut Vec<Value>, source: &str, disabled: bool, rule: &str) {
    if disabled {
        limitations.push(json!({
            "source": source,
            "code": "disabled_rule",
            "detail": format!("{rule} is disabled by the module rules"),
        }));
    }
}
