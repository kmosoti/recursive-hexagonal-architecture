//! Every crate-level rule of plan §5, on synthetic crate graphs (CHG-003).
//!
//! Synthetic, not generated from the corpus: `xtask/tests/corpus/manifest.toml`
//! is the input to CHG-004's harness, and a checker that reads the cases it is
//! judged on can satisfy one without implementing the rule. These graphs are
//! built here by hand, and the corpus grades the checker separately in W4.
//!
//! The seeded shapes deliberately mirror the corpus case ids, so that when W4
//! runs, a disagreement between these tests and the corpus is visible as a
//! disagreement rather than as two independent failures.

use std::path::PathBuf;

use xtask::graph::check::{Severity, check};
use xtask::graph::model::{CrateGraph, CrateNode, DepKind, Edge, MetadataMode, PortRef, Target};
use xtask::graph::rules::Rules;

/// Rules with the synthetic workspace's classification and nothing allowed.
fn rules(extra: &str) -> Rules {
    let text = format!(
        "schema_version = 1\n\
         [classification]\n\
         adapter_prefix = \"adapter-\"\n\
         app_prefix = \"app-\"\n\
         tools = [\"tool-t\"]\n\
         harness = []\n\
         [core]\n\
         allow = []\n\
         dev_allow = []\n\
         allow_build_scripts = false\n\
         [adapters]\n\
         require_port_owner_dependency = true\n\
         foreign_core_dependency = \"warn\"\n\
         [transitive]\n\
         enabled = false\n\
         {extra}"
    );
    Rules::parse(&text).expect("the synthetic rules parse")
}

fn node(name: &str, role: Option<&str>) -> CrateNode {
    CrateNode {
        name: name.to_owned(),
        manifest_path: PathBuf::from(format!("/w/{name}/Cargo.toml")),
        role: None,
        role_source: None,
        declared_role: role.map(ToOwned::to_owned),
        implements: Vec::new(),
        has_build_script: false,
    }
}

fn core(name: &str) -> CrateNode {
    node(name, Some("core"))
}

struct Builder {
    crates: Vec<CrateNode>,
    edges: Vec<Edge>,
}

impl Builder {
    fn new(crates: Vec<CrateNode>) -> Self {
        Self {
            crates,
            edges: Vec::new(),
        }
    }

    fn edge(mut self, from: &str, to: &str, kind: DepKind) -> Self {
        let member = self.crates.iter().any(|c| c.name == to);
        self.edges.push(Edge {
            from: from.to_owned(),
            to: if member {
                Target::Member {
                    name: to.to_owned(),
                }
            } else {
                Target::External {
                    name: to.to_owned(),
                }
            },
            kind,
            optional: false,
            target_cfg: None,
            rename: None,
        });
        self
    }

    fn shaped(mut self, from: &str, to: &str, shape: impl FnOnce(&mut Edge)) -> Self {
        self = self.edge(from, to, DepKind::Normal);
        shape(self.edges.last_mut().expect("just pushed"));
        self
    }

    fn build(self) -> CrateGraph {
        CrateGraph {
            workspace_root: PathBuf::from("/w"),
            crates: self.crates,
            edges: self.edges,
            mode: MetadataMode::NoDeps,
        }
    }
}

/// The rule ids of every error finding, in order.
fn error_rules(graph: &CrateGraph, rules: &Rules) -> Vec<&'static str> {
    check(graph, rules)
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.rule)
        .collect()
}

// ---------------------------------------------------------------- direction

#[test]
fn c01_a_core_depending_on_an_adapter_is_reported() {
    let graph = Builder::new(vec![core("core-a"), node("adapter-x", None)])
        .edge("core-a", "adapter-x", DepKind::Normal)
        .build();
    let outcome = check(&graph, &rules(""));
    assert_eq!(error_rules(&graph, &rules("")), vec!["dir.core_to_adapter"]);

    // The witness must let a reader find the edge without the checker.
    let finding = &outcome.findings[0];
    assert_eq!(
        (finding.from.as_str(), finding.to.as_deref()),
        ("core-a", Some("adapter-x"))
    );
    let witness = finding.witness.as_ref().expect("a witness");
    assert_eq!(witness.edge, "core-a -> adapter-x (normal)");
    assert_eq!(witness.declared_in, "[dependencies]");
    assert!(finding.manifest_path.ends_with("core-a/Cargo.toml"));
}

#[test]
fn c02_a_core_depending_on_a_composition_root_is_reported() {
    let graph = Builder::new(vec![core("core-a"), node("app-main", None)])
        .edge("core-a", "app-main", DepKind::Normal)
        .build();
    assert_eq!(error_rules(&graph, &rules("")), vec!["dir.core_to_app"]);
}

#[test]
fn c06_and_c07_a_non_root_depending_on_an_adapter_is_reported() {
    // C06: adapter to adapter.
    let graph = Builder::new(vec![node("adapter-x", None), node("adapter-y", None)])
        .edge("adapter-x", "adapter-y", DepKind::Normal)
        .build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        vec!["dir.non_root_to_adapter"]
    );

    // C07: a tool, which the tools list names and the harness list does not.
    let graph = Builder::new(vec![node("tool-t", None), node("adapter-x", None)])
        .edge("tool-t", "adapter-x", DepKind::Normal)
        .build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        vec!["dir.non_root_to_adapter"]
    );
}

#[test]
fn c12_a_tool_that_is_depended_on_is_reported() {
    let graph = Builder::new(vec![core("core-a"), node("tool-t", None)])
        .edge("core-a", "tool-t", DepKind::Normal)
        .build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        vec!["dir.tool_depended_on"]
    );
}

#[test]
fn l05_a_composition_root_may_depend_on_everything() {
    let graph = Builder::new(vec![
        node("app-main", None),
        core("core-a"),
        node("adapter-x", None),
    ])
    .edge("app-main", "core-a", DepKind::Normal)
    .edge("app-main", "adapter-x", DepKind::Normal)
    .build();
    assert_eq!(error_rules(&graph, &rules("")), Vec::<&str>::new());
}

#[test]
fn l06_a_listed_harness_may_depend_on_an_adapter() {
    let graph = Builder::new(vec![node("harness-h", None), node("adapter-x", None)])
        .edge("harness-h", "adapter-x", DepKind::Normal)
        .build();
    let rules = rules("");
    // Not in the harness list, and no prefix or metadata classifies it, so
    // the crate is unclassified — and classification comes first. An
    // unclassified crate is not judged for direction, because there is no
    // role to judge it against; class.unclassified is the finding, and it is
    // the one a reader can act on.
    assert_eq!(error_rules(&graph, &rules), vec!["class.unclassified"]);

    let listed = Rules::parse(
        "schema_version = 1\n[classification]\nadapter_prefix = \"adapter-\"\napp_prefix = \"app-\"\ntools = []\nharness = [\"harness-h\"]\n",
    )
    .expect("parses");
    assert_eq!(error_rules(&graph, &listed), Vec::<&str>::new());
}

#[test]
fn l07_a_dev_dependency_on_an_adapter_is_listed_and_never_an_alarm() {
    // The one legitimate case that is also a reported fact. Silence and a
    // listed fact are different outcomes.
    let graph = Builder::new(vec![core("core-a"), node("adapter-x", None)])
        .edge("core-a", "adapter-x", DepKind::Dev)
        .build();
    let outcome = check(&graph, &rules(""));
    assert_eq!(outcome.errors(), 0, "a dev edge must not fail D5");
    assert_eq!(outcome.harness_edges.len(), 1);
    let listed = &outcome.harness_edges[0];
    assert_eq!(listed.rule, "dir.non_root_to_adapter");
    assert_eq!(listed.severity, Severity::Note);
    assert_eq!(listed.kind, Some("dev"));
}

/// A named way of spelling one dependency edge (corpus C15 to C18).
type Shape = (&'static str, fn(&mut Edge));

// ------------------------------------------------------------- the C15..C18
// One edge, four spellings. A checker reading Cargo.toml text instead of
// cargo metadata misses one of them; all four must report identically.

#[test]
fn c15_to_c18_the_same_edge_is_found_behind_every_spelling() {
    let shapes: [Shape; 4] = [
        ("C15 target cfg", |e| {
            e.target_cfg = Some("cfg(unix)".to_owned());
        }),
        ("C16 optional", |e| e.optional = true),
        ("C17 renamed", |e| e.rename = Some("ax".to_owned())),
        // C18 workspace inheritance arrives from cargo metadata as an
        // ordinary entry, which is the point: nothing to special-case.
        ("C18 inherited", |_| {}),
    ];
    for (name, shape) in shapes {
        let graph = Builder::new(vec![core("core-a"), node("adapter-x", None)])
            .shaped("core-a", "adapter-x", shape)
            .build();
        let outcome = check(&graph, &rules(""));
        assert_eq!(
            outcome.findings.iter().map(|f| f.rule).collect::<Vec<_>>(),
            vec!["dir.core_to_adapter"],
            "{name}"
        );
        // C17: reported under the package name, never the rename.
        assert_eq!(
            outcome.findings[0].to.as_deref(),
            Some("adapter-x"),
            "{name}"
        );
    }
}

#[test]
fn c17_the_witness_still_records_the_key_the_manifest_uses() {
    let graph = Builder::new(vec![core("core-a"), node("adapter-x", None)])
        .shaped("core-a", "adapter-x", |e| {
            e.rename = Some("ax".to_owned());
        })
        .build();
    let outcome = check(&graph, &rules(""));
    let witness = outcome.findings[0].witness.as_ref().expect("a witness");
    assert_eq!(witness.rename.as_deref(), Some("ax"));
}

// ------------------------------------------------------------------- effect

#[test]
fn c03_and_c21_an_external_dependency_of_a_core_must_be_allow_listed() {
    let normal = Builder::new(vec![core("core-a")])
        .edge("core-a", "tokio", DepKind::Normal)
        .build();
    assert_eq!(
        error_rules(&normal, &rules("")),
        vec!["effect.core_disallowed_dependency"]
    );

    let dev = Builder::new(vec![core("core-a")])
        .edge("core-a", "tokio", DepKind::Dev)
        .build();
    assert_eq!(
        error_rules(&dev, &rules("")),
        vec!["effect.core_disallowed_dev_dependency"]
    );
}

#[test]
fn l03_l04_and_l08_allow_lists_and_adapters_are_respected() {
    let allowed = rules("");
    let with_allow = Rules::parse(
        "schema_version = 1\n[classification]\nadapter_prefix = \"adapter-\"\napp_prefix = \"app-\"\n[core]\nallow = [\"sha2\"]\ndev_allow = [\"proptest\"]\n",
    )
    .expect("parses");

    let graph = Builder::new(vec![core("core-a")])
        .edge("core-a", "sha2", DepKind::Normal)
        .edge("core-a", "proptest", DepKind::Dev)
        .build();
    assert_eq!(error_rules(&graph, &with_allow), Vec::<&str>::new());
    assert_eq!(
        error_rules(&graph, &allowed).len(),
        2,
        "empty allow-lists deny both"
    );

    // L08: an adapter may depend on anything. Effects are what it is for.
    let adapter = Builder::new(vec![node("adapter-x", None)])
        .edge("adapter-x", "tokio", DepKind::Normal)
        .build();
    assert_eq!(error_rules(&adapter, &allowed), Vec::<&str>::new());
}

#[test]
fn c04_a_core_with_a_build_script_is_reported() {
    let mut crate_a = core("core-a");
    crate_a.has_build_script = true;
    let graph = Builder::new(vec![crate_a]).build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        vec!["effect.core_build_script"]
    );

    let permitted =
        Rules::parse("schema_version = 1\n[classification]\n[core]\nallow_build_scripts = true\n")
            .expect("parses");
    // Still classified: an explicit role, so no class.unclassified either.
    assert_eq!(error_rules(&graph, &permitted), Vec::<&str>::new());
}

// ----------------------------------------------------------------- forbidden

#[test]
fn c08_a_forbidden_edge_matches_a_trailing_wildcard() {
    let forbidden = Rules::parse(
        "schema_version = 1\n[classification]\n[core]\nallow = [\"aws-sdk-s3\"]\n[[forbidden]]\nfrom = \"core-a\"\nto = \"aws-sdk-*\"\nreason = \"no object store in a core\"\n",
    )
    .expect("parses");
    let graph = Builder::new(vec![core("core-a")])
        .edge("core-a", "aws-sdk-s3", DepKind::Normal)
        .build();
    let outcome = check(&graph, &forbidden);
    assert_eq!(
        outcome.findings.iter().map(|f| f.rule).collect::<Vec<_>>(),
        vec!["forbidden.edge"]
    );
    assert!(
        outcome.findings[0]
            .message
            .contains("no object store in a core"),
        "the reason belongs in the message: {}",
        outcome.findings[0].message
    );
}

#[test]
fn c09_a_role_selector_forbids_the_edge_for_every_crate_of_that_role() {
    let forbidden = Rules::parse(
        "schema_version = 1\n[classification]\n[core]\nallow = [\"tokio\"]\n[[forbidden]]\nfrom = \"role:core\"\nto = \"tokio\"\n",
    )
    .expect("parses");
    let graph = Builder::new(vec![core("core-a"), core("core-b")])
        .edge("core-b", "tokio", DepKind::Normal)
        .build();
    // tokio is allow-listed, so only the forbidden rule can fire.
    assert_eq!(error_rules(&graph, &forbidden), vec!["forbidden.edge"]);
}

// ------------------------------------------------------------- adapters/meta

#[test]
fn c13_an_adapter_that_cannot_name_its_port_owner_is_reported() {
    let mut adapter = node("adapter-x", None);
    adapter.implements = vec![PortRef::parse("core-a::Port")];
    let graph = Builder::new(vec![adapter, core("core-a"), core("core-b")])
        .edge("adapter-x", "core-b", DepKind::Normal)
        .build();
    let found = error_rules(&graph, &rules(""));
    assert!(found.contains(&"adapter.missing_port_owner"), "{found:?}");
}

#[test]
fn c14_a_port_owner_reached_only_as_a_dev_dependency_is_reported() {
    let mut adapter = node("adapter-x", None);
    adapter.implements = vec![PortRef::parse("core-a::Port")];
    let graph = Builder::new(vec![adapter, core("core-a")])
        .edge("adapter-x", "core-a", DepKind::Dev)
        .build();
    let found = error_rules(&graph, &rules(""));
    assert!(
        found.contains(&"adapter.port_owner_wrong_kind"),
        "{found:?}"
    );
}

#[test]
fn c20_a_port_whose_owner_is_not_a_member_is_reported() {
    let mut adapter = node("adapter-x", None);
    adapter.implements = vec![PortRef::parse("ghost::Port")];
    let graph = Builder::new(vec![adapter]).build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        vec!["meta.unknown_port_owner"]
    );
}

#[test]
fn l01_and_l09_an_adapter_depending_on_every_owner_it_implements_is_silent() {
    let mut adapter = node("adapter-x", None);
    adapter.implements = vec![
        PortRef::parse("core-a::Reader"),
        PortRef::parse("core-b::Writer"),
    ];
    let graph = Builder::new(vec![adapter, core("core-a"), core("core-b")])
        .edge("adapter-x", "core-a", DepKind::Normal)
        .edge("adapter-x", "core-b", DepKind::Normal)
        .build();
    let outcome = check(&graph, &rules(""));
    assert_eq!(outcome.errors(), 0);
    assert_eq!(
        outcome.warnings(),
        0,
        "every core it depends on is one it implements"
    );
}

#[test]
fn an_adapter_using_a_core_whose_port_it_does_not_implement_is_a_warning() {
    let mut adapter = node("adapter-x", None);
    adapter.implements = vec![PortRef::parse("core-a::Port")];
    let graph = Builder::new(vec![adapter, core("core-a"), core("core-b")])
        .edge("adapter-x", "core-a", DepKind::Normal)
        .edge("adapter-x", "core-b", DepKind::Normal)
        .build();
    let outcome = check(&graph, &rules(""));
    assert_eq!(outcome.errors(), 0, "a warning must not fail the check");
    assert_eq!(outcome.warnings(), 1);
    assert_eq!(outcome.findings[0].rule, "adapter.foreign_core");

    let strict = Rules::parse(
        "schema_version = 1\n[classification]\nadapter_prefix = \"adapter-\"\n[adapters]\nforeign_core_dependency = \"error\"\n",
    )
    .expect("parses");
    assert_eq!(
        check(&graph, &strict).errors(),
        1,
        "configurable to an error"
    );
}

// --------------------------------------------------------- classification

#[test]
fn c10_and_c11_classification_problems_are_reported() {
    let graph = Builder::new(vec![node("mystery", None)]).build();
    assert_eq!(error_rules(&graph, &rules("")), vec!["class.unclassified"]);

    let graph = Builder::new(vec![node("adapter-x", Some("core"))]).build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        vec!["class.prefix_role_conflict"]
    );
}

#[test]
fn l11_a_prefix_inside_a_name_is_not_a_prefix() {
    let graph = Builder::new(vec![node("planner-adapter-utils", Some("core"))]).build();
    assert_eq!(
        error_rules(&graph, &rules("")),
        Vec::<&str>::new(),
        "a substring match would report class.prefix_role_conflict here"
    );
}

// ------------------------------------------------------------------ cycles

#[test]
fn c05_a_package_cycle_is_reported_once_with_its_path() {
    let graph = Builder::new(vec![core("core-a"), core("core-b")])
        .edge("core-a", "core-b", DepKind::Normal)
        .edge("core-b", "core-a", DepKind::Normal)
        .build();
    let outcome = check(&graph, &rules(""));
    let cycles: Vec<_> = outcome
        .findings
        .iter()
        .filter(|f| f.rule == "graph.cycle")
        .collect();
    assert_eq!(
        cycles.len(),
        1,
        "one report per cycle, not one per entry point"
    );
    let witness = cycles[0].witness.as_ref().expect("a witness");
    assert!(
        witness.edge.contains("core-a") && witness.edge.contains("core-b"),
        "the witness names the path: {}",
        witness.edge
    );
}

#[test]
fn l02_a_declared_acyclic_edge_between_cores_is_silent() {
    let graph = Builder::new(vec![core("core-a"), core("core-b")])
        .edge("core-b", "core-a", DepKind::Normal)
        .build();
    assert_eq!(error_rules(&graph, &rules("")), Vec::<&str>::new());
}

#[test]
fn a_dev_dependency_pointing_back_is_not_a_cycle() {
    // cargo permits it, and the declared graph B3 talks about is the normal
    // one. Reporting it would fail L07's shape for the wrong reason.
    let graph = Builder::new(vec![core("core-a"), core("core-b")])
        .edge("core-a", "core-b", DepKind::Normal)
        .edge("core-b", "core-a", DepKind::Dev)
        .build();
    let outcome = check(&graph, &rules(""));
    assert!(
        !outcome.findings.iter().any(|f| f.rule == "graph.cycle"),
        "{:?}",
        outcome.findings
    );
}

// ------------------------------------------------------------- limitations

#[test]
fn a_rule_that_cannot_be_evaluated_is_a_limitation_not_a_silence() {
    // EM-C01's hole, stated by the run rather than left to be inferred from
    // an empty findings list.
    let graph = Builder::new(vec![core("core-a")]).build();
    let outcome = check(&graph, &rules(""));
    assert!(
        outcome
            .limitations
            .iter()
            .any(|l| l.contains("transitive.core_disallowed_dependency")),
        "{:?}",
        outcome.limitations
    );
}

#[test]
fn em_c01_a_requested_transitive_rule_that_did_not_run_is_still_a_limitation() {
    // [transitive] enabled = true asks for a rule this version cannot
    // evaluate. The limitation must survive the request: a clean summary with
    // no limitation would read as a pass over transitive dependencies (review
    // finding on pull request 6). `architecture::run` refuses the
    // configuration outright with exit 2; this is the in-process guard.
    let graph = Builder::new(vec![core("core-a")]).build();
    let rules = Rules::parse(
        "schema_version = 1\n[classification]\nadapter_prefix = \"adapter-\"\n[transitive]\nenabled = true\n",
    )
    .expect("parses");
    let outcome = check(&graph, &rules);
    assert!(
        outcome.limitations.iter().any(|l| {
            l.contains("transitive.core_disallowed_dependency") && l.contains("not evaluated")
        }),
        "{:?}",
        outcome.limitations
    );
}

/// The checker must not read the corpus it is judged on (CHG-003 non-goal).
///
/// Scope is the checker: `graph/`, `metadata.rs`, `architecture.rs`. Two other
/// places in xtask name the manifest legitimately and are not checked here.
/// `corpus.rs` holds the manifest's own types, and CHG-004's harness will read
/// it from there. `evidence/mod.rs` lists it among the fixtures whose digests
/// go into an evidence record — recording what a run was configured by is the
/// opposite of consulting the answers, and a record that omitted it would be
/// less honest, not more.
#[test]
fn the_checker_never_reads_the_corpus_manifest() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let checker: Vec<PathBuf> = walk(&src)
        .into_iter()
        .filter(|p| {
            p.ends_with("metadata.rs")
                || p.ends_with("architecture.rs")
                || p.components().any(|c| c.as_os_str() == "graph")
        })
        .collect();
    assert!(
        checker.len() >= 6,
        "the checker's modules should all be found, got {checker:?}"
    );
    let mut offenders = Vec::new();
    for entry in checker {
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        // Comments may DISCUSS the manifest — several say why the checker
        // must not read it. Only code counts, so comment lines are stripped
        // before looking.
        let code: String = text
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("//") && !t.starts_with('*') && !t.starts_with("/*")
            })
            .collect::<Vec<_>>()
            .join("\n");
        if code.contains("corpus/manifest.toml") {
            offenders.push(entry.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "these name the corpus manifest, which only the CHG-004 harness may read: {offenders:?}"
    );
}

fn walk(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
