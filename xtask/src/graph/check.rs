//! Evaluating the rules of plan §5 against a crate graph.
//!
//! Every finding carries a witness: the concrete edge, the manifest path, and
//! the section the edge is declared in, so a reader can open the file and see
//! the same thing the checker saw (§9.3).
//!
//! Two behaviours are worth stating because they are easy to get wrong and
//! the corpus pins them:
//!
//! * **Dev-kind edges are excluded from D5 and listed instead.** A crate's own
//!   tests are a harness, so `core-a` taking `adapter-x` as a dev-dependency
//!   is legitimate — but it is still reported, under `harness_edges`, because
//!   silence and a listed fact are different outcomes (corpus L07).
//! * **A rule that cannot be evaluated says so.** `transitive.*` needs
//!   resolved metadata; in `--no-deps` mode it is a limitation, never an
//!   absence of findings (corpus EM-C01).

use crate::graph::classify::{self, Problem};
use crate::graph::model::{CrateGraph, DepKind, Edge, Role, RoleSource, Target};
use crate::graph::rules::{Rules, glob_matches, selector_matches};

/// How much a finding matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Fails the check; exit code 1.
    Error,
    /// Reported, does not fail.
    Warning,
    /// A fact the report must carry without it being a complaint.
    Note,
}

/// One violation, with the witness that shows it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    pub from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    pub manifest_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<Witness>,
    pub message: String,
    /// Structured rule-specific facts; rendered without consulting any corpus.
    #[serde(flatten)]
    pub details: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Where the thing the finding is about is written down.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Witness {
    /// `core-a -> adapter-x (normal)`.
    pub edge: String,
    /// `[dependencies]`, `[target.'cfg(unix)'.dependencies]`, …
    pub declared_in: String,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The key used in `Cargo.toml`, when it differs from the package name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename: Option<String>,
}

impl Finding {
    fn detail(mut self, key: &str, value: impl serde::Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.details.insert(key.to_owned(), value);
        }
        self
    }
}

impl Witness {
    fn of(edge: &Edge) -> Self {
        Self {
            edge: format!(
                "{} -> {} ({})",
                edge.from,
                edge.to.name(),
                edge.kind.as_str()
            ),
            declared_in: edge.declared_in(),
            optional: edge.optional,
            target: edge.target_cfg.clone(),
            rename: edge.rename.clone(),
        }
    }
}

/// What a classified crate looks like in the report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ClassifiedCrate {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_source: Option<RoleSource>,
    pub manifest_path: String,
}

/// Everything one run of the checker observed.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub findings: Vec<Finding>,
    pub classification: Vec<ClassifiedCrate>,
    /// Facts the report carries without them being complaints: the dev-kind
    /// edges D5 excludes.
    pub harness_edges: Vec<Finding>,
    /// What this run could not check, and why.
    pub limitations: Vec<String>,
}

impl Outcome {
    #[must_use]
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    #[must_use]
    pub fn warnings(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count()
    }
}

fn manifest_of(graph: &CrateGraph, name: &str) -> String {
    graph
        .crate_named(name)
        .map(|c| c.manifest_path.display().to_string())
        .unwrap_or_default()
}

fn edge_finding(
    rule: &'static str,
    severity: Severity,
    graph: &CrateGraph,
    edge: &Edge,
    message: String,
) -> Finding {
    Finding {
        details: std::collections::BTreeMap::new(),
        rule,
        severity,
        from: edge.from.clone(),
        to: Some(edge.to.name().to_owned()),
        kind: Some(edge.kind.as_str()),
        manifest_path: manifest_of(graph, &edge.from),
        witness: Some(Witness::of(edge)),
        message,
    }
}

/// Runs every crate-level rule.
#[must_use]
pub fn check(graph: &CrateGraph, rules: &Rules) -> Outcome {
    let mut outcome = Outcome::default();
    let graph = &classified(graph, rules, &mut outcome);

    for edge in &graph.edges {
        let from_role = graph.role_of(&edge.from);
        direction_rules(graph, edge, from_role, &mut outcome);
        effect_rules(graph, rules, edge, from_role, &mut outcome);
        forbidden_rules(graph, rules, edge, from_role, &mut outcome);
    }

    adapter_rules(graph, rules, &mut outcome);
    cycles(graph, &mut outcome);

    // No transitive evaluator exists in this version: the graph is always
    // read with `cargo metadata --no-deps`. The limitation is stated whether
    // or not [transitive] enabled asks for more, because a requested rule
    // that did not run is the one most likely to be mistaken for a clean
    // result (review finding on pull request 6, CHG-003.1). `architecture::run`
    // refuses the configuration outright; this keeps in-process callers honest.
    outcome.limitations.push(if rules.transitive.enabled {
        "transitive.core_disallowed_dependency: requested by [transitive] enabled = true and \
         not evaluated. This version reads `cargo metadata --no-deps` only and has no \
         transitive evaluator (L2, after CHG-004); the summary is not a pass over transitive \
         dependencies."
            .to_owned()
    } else {
        "transitive.core_disallowed_dependency: not evaluated. The graph is read with \
         `cargo metadata --no-deps`, which shows direct declared edges only (spec §4.1, \
         Law 5 row). No version yet honours [transitive] enabled = true."
            .to_owned()
    });
    outcome
}

/// Classifies every member, recording the result and any classification
/// finding. Returns a graph whose nodes carry their roles.
fn classified(graph: &CrateGraph, rules: &Rules, outcome: &mut Outcome) -> CrateGraph {
    let mut out = graph.clone();
    for node in &mut out.crates {
        let result = classify::classify(node, &rules.classification);
        node.role = result.role;
        node.role_source = result.source;
        let manifest_path = node.manifest_path.display().to_string();
        match result.problem {
            Some(Problem::Unclassified) => outcome.findings.push(Finding {
                details: std::collections::BTreeMap::new(),
                rule: "class.unclassified",
                severity: Severity::Error,
                from: node.name.clone(),
                to: None,
                kind: None,
                manifest_path: manifest_path.clone(),
                witness: None,
                message: format!(
                    "{}: no [package.metadata.rha] role, no adapter- or app- prefix, and not in \
                     the tools or harness lists. Classification is total (§11.2): a member with \
                     no role is not given one.",
                    node.name
                ),
            }),
            Some(Problem::PrefixRoleConflict {
                prefix_says,
                metadata_says,
            }) => outcome.findings.push(Finding {
                details: std::collections::BTreeMap::from([
                    ("prefix_says".to_owned(), serde_json::json!(prefix_says)),
                    ("metadata_says".to_owned(), serde_json::json!(metadata_says)),
                ]),
                rule: "class.prefix_role_conflict",
                severity: Severity::Error,
                from: node.name.clone(),
                to: None,
                kind: None,
                manifest_path: manifest_path.clone(),
                witness: None,
                message: format!(
                    "{}: the name says {}, [package.metadata.rha] role says {}. The metadata \
                     wins, but one of the two is wrong and the crate cannot say which.",
                    node.name,
                    prefix_says.as_str(),
                    metadata_says.as_str()
                ),
            }),
            Some(Problem::UnknownRole { declared }) => outcome.findings.push(Finding {
                details: std::collections::BTreeMap::new(),
                rule: "class.unclassified",
                severity: Severity::Error,
                from: node.name.clone(),
                to: None,
                kind: None,
                manifest_path: manifest_path.clone(),
                witness: None,
                message: format!(
                    "{}: [package.metadata.rha] role = \"{declared}\" is not one of core, \
                     adapter, app, tool, harness, so nothing classified the crate.",
                    node.name
                ),
            }),
            None => {}
        }
        outcome.classification.push(ClassifiedCrate {
            name: node.name.clone(),
            role: node.role,
            role_source: node.role_source,
            manifest_path,
        });
    }
    out
}

/// `dir.*`: who may depend on whom.
fn direction_rules(
    graph: &CrateGraph,
    edge: &Edge,
    from_role: Option<Role>,
    outcome: &mut Outcome,
) {
    let Some(to_role) = (match &edge.to {
        crate::graph::model::Target::Member { name } => graph.role_of(name),
        crate::graph::model::Target::External { .. } => None,
    }) else {
        return;
    };
    let Some(from_role) = from_role else { return };

    // A dev-kind edge to an adapter is a test harness, whatever the depending
    // crate's role: a crate's own tests construct adapters, which is what
    // adapters are for. Plan §8 W3 excludes dev-kind edges from D5, and the
    // same reasoning covers D1 — corpus L07 seeds exactly this edge from a
    // core and expects no alarm. It is still listed, because silence and a
    // listed fact are different outcomes.
    if to_role == Role::Adapter && edge.kind == DepKind::Dev {
        outcome.harness_edges.push(edge_finding(
            "dir.non_root_to_adapter",
            Severity::Note,
            graph,
            edge,
            format!(
                "listed, not an alarm: a dev-dependency on an adapter is a test harness \
                 (D1 and D5, §6.13): {} -> {}",
                edge.from,
                edge.to.name()
            ),
        ));
        return;
    }

    if from_role == Role::Core && to_role == Role::Adapter {
        outcome.findings.push(edge_finding(
            "dir.core_to_adapter",
            Severity::Error,
            graph,
            edge,
            format!(
                "core crates must not depend on adapter crates (D1, §6.13): {} -> {}",
                edge.from,
                edge.to.name()
            ),
        ));
        return;
    }
    if from_role == Role::Core && to_role == Role::App {
        outcome.findings.push(edge_finding(
            "dir.core_to_app",
            Severity::Error,
            graph,
            edge,
            format!(
                "a core must not depend on a composition root (D1, §6.13): {} -> {}",
                edge.from,
                edge.to.name()
            ),
        ));
        return;
    }
    if to_role == Role::Adapter && !from_role.may_depend_on_adapter() {
        // D5. Dev-kind edges returned above; only normal and build reach here.
        outcome.findings.push(edge_finding(
            "dir.non_root_to_adapter",
            Severity::Error,
            graph,
            edge,
            format!(
                "only composition roots and test harnesses may depend on an adapter \
                 (D5, §6.13): {} is {}, {} -> {}",
                edge.from,
                from_role.as_str(),
                edge.from,
                edge.to.name()
            ),
        ));
        return;
    }
    if to_role == Role::Tool {
        outcome.findings.push(edge_finding(
            "dir.tool_depended_on",
            Severity::Error,
            graph,
            edge,
            format!(
                "a tool is built by the repository, not depended on by it: {} -> {}",
                edge.from,
                edge.to.name()
            ),
        ));
    }
}

/// `effect.*`: what a core may reach.
fn effect_rules(
    graph: &CrateGraph,
    rules: &Rules,
    edge: &Edge,
    from_role: Option<Role>,
    outcome: &mut Outcome,
) {
    if from_role != Some(Role::Core) || edge.to.is_member() {
        return;
    }
    let name = edge.to.name();
    match edge.kind {
        DepKind::Dev => {
            if !rules.core.dev_allow.iter().any(|a| glob_matches(a, name)) {
                outcome.findings.push(edge_finding(
                    "effect.core_disallowed_dev_dependency",
                    Severity::Error,
                    graph,
                    edge,
                    format!(
                        "{name} is not in [core] dev_allow (Law 5, §6.8): {} -> {name} (dev)",
                        edge.from
                    ),
                ));
            }
        }
        DepKind::Normal | DepKind::Build => {
            if !rules.core.allow.iter().any(|a| glob_matches(a, name)) {
                outcome.findings.push(edge_finding(
                    "effect.core_disallowed_dependency",
                    Severity::Error,
                    graph,
                    edge,
                    format!(
                        "{name} is not in [core] allow (Law 5, §6.8): {} -> {name}",
                        edge.from
                    ),
                ));
            }
        }
    }
}

/// `forbidden.edge`: an edge a rule names outright.
fn forbidden_rules(
    graph: &CrateGraph,
    rules: &Rules,
    edge: &Edge,
    from_role: Option<Role>,
    outcome: &mut Outcome,
) {
    for rule in &rules.forbidden {
        if selector_matches(&rule.from, &edge.from, from_role)
            && glob_matches(&rule.to, edge.to.name())
        {
            let reason = rule
                .reason
                .clone()
                .unwrap_or_else(|| "forbidden by rha-crates.toml".to_owned());
            outcome.findings.push(
                edge_finding(
                    "forbidden.edge",
                    Severity::Error,
                    graph,
                    edge,
                    format!(
                        "{} -> {} matches the forbidden rule {} -> {}: {reason}",
                        edge.from,
                        edge.to.name(),
                        rule.from,
                        rule.to
                    ),
                )
                .detail("matched_rule", format!("{} -> {}", rule.from, rule.to)),
            );
        }
    }
}

/// `adapter.*` and `meta.*`: an adapter against the ports it claims.
fn adapter_rules(graph: &CrateGraph, rules: &Rules, outcome: &mut Outcome) {
    for node in &graph.crates {
        let manifest_path = node.manifest_path.display().to_string();
        for port in &node.implements {
            let owner_is_member = graph.crate_named(&port.owner).is_some();
            if !owner_is_member {
                outcome.findings.push(Finding {
                    details: std::collections::BTreeMap::from([
                        ("port".to_owned(), serde_json::json!(port.path)),
                        ("owner".to_owned(), serde_json::json!(port.owner)),
                    ]),
                    rule: "meta.unknown_port_owner",
                    severity: Severity::Error,
                    from: node.name.clone(),
                    to: Some(port.owner.clone()),
                    kind: None,
                    manifest_path: manifest_path.clone(),
                    witness: None,
                    message: format!(
                        "{} declares implements = [\"{}\"], and no member named {} exists",
                        node.name, port.path, port.owner
                    ),
                });
                continue;
            }
            if !rules.adapters.require_port_owner_dependency {
                continue;
            }
            // The owner must be the workspace member named by the port. An
            // external package with the same name cannot provide the member's
            // port, and a normal member edge must win over a dev/build edge
            // regardless of declaration order.
            let is_owner =
                |e: &&Edge| matches!(&e.to, Target::Member { name } if name == &port.owner);
            let normal = graph
                .edges_from(&node.name)
                .filter(is_owner)
                .find(|e| e.kind == DepKind::Normal);
            let other = graph
                .edges_from(&node.name)
                .filter(is_owner)
                .find(|e| e.kind != DepKind::Normal);
            match normal.or(other) {
                Some(e) if e.kind == DepKind::Normal => {}
                Some(e) => outcome.findings.push(
                    edge_finding(
                        "adapter.port_owner_wrong_kind",
                        Severity::Error,
                        graph,
                        e,
                        format!(
                            "{} implements {} but depends on {} only as a {}-dependency, so the \
                         implementation does not compile outside that context",
                            node.name,
                            port.path,
                            port.owner,
                            e.kind.as_str()
                        ),
                    )
                    .detail("port", &port.path)
                    .detail("owner", &port.owner),
                ),
                None => outcome.findings.push(Finding {
                    details: std::collections::BTreeMap::from([
                        ("port".to_owned(), serde_json::json!(port.path)),
                        ("owner".to_owned(), serde_json::json!(port.owner)),
                    ]),
                    rule: "adapter.missing_port_owner",
                    severity: Severity::Error,
                    from: node.name.clone(),
                    to: Some(port.owner.clone()),
                    kind: None,
                    manifest_path: manifest_path.clone(),
                    witness: None,
                    message: format!(
                        "{} implements {} and does not depend on {}, so it cannot name the port \
                         it claims to implement",
                        node.name, port.path, port.owner
                    ),
                }),
            }
        }

        build_script_and_foreign_core(graph, rules, node, &manifest_path, outcome);
    }
}

/// The two per-crate rules that are not about declared ports: a core with a
/// build script, and an adapter reaching a core whose port it does not
/// implement. Split from [`adapter_rules`] to keep each readable.
fn build_script_and_foreign_core(
    graph: &CrateGraph,
    rules: &Rules,
    node: &crate::graph::model::CrateNode,
    manifest_path: &str,
    outcome: &mut Outcome,
) {
    {
        // effect.core_build_script: a build script runs arbitrary code at
        // build time, which the Clippy deny list cannot see.
        if node.role == Some(Role::Core) && node.has_build_script && !rules.core.allow_build_scripts
        {
            outcome.findings.push(Finding {
                details: std::collections::BTreeMap::new(),
                rule: "effect.core_build_script",
                severity: Severity::Error,
                from: node.name.clone(),
                to: None,
                kind: None,
                manifest_path: manifest_path.to_owned(),
                witness: None,
                message: format!(
                    "{} is a core crate with a build script, and [core] allow_build_scripts is \
                     false: a build script is an ambient effect (Law 5, §6.8)",
                    node.name
                ),
            });
        }

        // adapter.foreign_core: an adapter depending on a core whose port it
        // does not implement. A warning by default, because using a shared
        // core's types is legitimate; D1 constrains the direction, not the
        // existence of the edge.
        if node.role == Some(Role::Adapter) {
            let severity = if rules.foreign_core_is_error() {
                Severity::Error
            } else {
                Severity::Warning
            };
            for edge in graph.edges_from(&node.name) {
                if edge.kind != DepKind::Normal || !edge.to.is_member() {
                    continue;
                }
                let target = edge.to.name();
                if graph.role_of(target) != Some(Role::Core) {
                    continue;
                }
                if node.implements.iter().any(|p| p.owner == target) {
                    continue;
                }
                outcome.findings.push(edge_finding(
                    "adapter.foreign_core",
                    severity,
                    graph,
                    edge,
                    format!(
                        "{} depends on the core {target} without implementing any of its ports",
                        node.name
                    ),
                ));
            }
        }
    }
}

/// `graph.cycle`: our own DFS over member edges.
///
/// Cargo rejects a package cycle while resolving, so a cyclic workspace often
/// fails before the checker sees it. That does not make this rule redundant:
/// when the graph does load, the cycle is ours to find and to name, and the
/// corpus case C05 requires this rule's own finding rather than accepting a
/// cargo error in its place.
fn cycles(graph: &CrateGraph, outcome: &mut Outcome) {
    /// 0 unvisited, 1 on the current path, 2 finished.
    type State<'a> = std::collections::BTreeMap<&'a str, u8>;

    fn visit<'a>(
        node: &'a str,
        graph: &'a CrateGraph,
        state: &mut State<'a>,
        path: &mut Vec<&'a str>,
        seen: &mut std::collections::BTreeSet<Vec<&'a str>>,
        found: &mut Vec<Vec<&'a str>>,
    ) {
        state.insert(node, 1);
        path.push(node);
        for edge in graph.edges_from(node) {
            // Only member edges can cycle. A dev-dependency pointing back is
            // legal in cargo and is not a cycle in the declared graph.
            if !edge.to.is_member() || edge.kind == DepKind::Dev {
                continue;
            }
            let Some(target) = graph.crate_named(edge.to.name()).map(|c| c.name.as_str()) else {
                continue;
            };
            match state.get(target).copied().unwrap_or(0) {
                0 => visit(target, graph, state, path, seen, found),
                1 => {
                    let start = path.iter().position(|n| *n == target).unwrap_or(0);
                    let mut cycle: Vec<&str> = path[start..].to_vec();
                    // The dedup key is the member SET, so the same cycle
                    // discovered from a different entry point is reported
                    // once. The reported path keeps traversal order, which is
                    // what a reader needs to follow it.
                    let mut key = cycle.clone();
                    key.sort_unstable();
                    if seen.insert(key) {
                        cycle.push(target);
                        found.push(cycle);
                    }
                }
                _ => {}
            }
        }
        path.pop();
        state.insert(node, 2);
    }

    let mut state: State = graph
        .crates
        .iter()
        .map(|c| (c.name.as_str(), 0u8))
        .collect();
    let mut path = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut found: Vec<Vec<&str>> = Vec::new();
    let names: Vec<&str> = graph.crates.iter().map(|c| c.name.as_str()).collect();
    for name in names {
        if state.get(name).copied().unwrap_or(0) == 0 {
            visit(name, graph, &mut state, &mut path, &mut seen, &mut found);
        }
    }

    for cycle in found {
        let joined = cycle.join(" -> ");
        let first = cycle.first().copied().unwrap_or_default();
        outcome.findings.push(Finding {
            details: std::collections::BTreeMap::from([(
                "members".to_owned(),
                serde_json::json!(
                    cycle
                        .iter()
                        .copied()
                        .collect::<std::collections::BTreeSet<_>>()
                ),
            )]),
            rule: "graph.cycle",
            severity: Severity::Error,
            from: first.to_owned(),
            to: None,
            kind: None,
            manifest_path: manifest_of(graph, first),
            witness: Some(Witness {
                edge: format!("cycle: {joined}"),
                declared_in: "[dependencies] of each member in the cycle".to_owned(),
                optional: false,
                target: None,
                rename: None,
            }),
            message: format!("the declared crate graph must be acyclic (B3, Law 6): {joined}"),
        });
    }
}
