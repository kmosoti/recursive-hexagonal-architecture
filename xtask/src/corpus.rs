//! The pre-registered H4 corpus manifest (CHG-002; plan §6).
//!
//! This module owns the manifest's shape and validation. CHG-004 adds the
//! crate-level generator and runner; module execution arrives in CHG-007.
//!
//! The manifest is committed before any checker code, so that a case cannot be
//! written to fit what a checker turned out to do (spec §17, §9.14). The
//! validation here is what keeps that promise mechanical rather than
//! aspirational: it fails on a repeated id, an unknown rule or cell, an
//! expected miss that cites no hole, and a declared case that carries no
//! workspace for CHG-004 to generate.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

pub mod fixture;
pub mod grade;
pub mod held_out;
pub mod runner;

/// Where the manifest lives, relative to the workspace root.
pub const MANIFEST_PATH: &str = "xtask/tests/corpus/manifest.toml";

/// The level a case is checked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// `cargo metadata` over a fixture workspace; no compilation.
    Crate,
    /// An extracted module graph inside one crate.
    Module,
}

/// What the corpus expects a run to observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expected {
    /// The violation must be reported.
    Detect,
    /// Nothing may be reported.
    NoAlarm,
    /// The violation is real and out of reach; the case cites the §4.1 hole.
    ExpectedMiss,
    /// A second opinion from a tool this program did not write. Recorded and
    /// explained, never scored against the others.
    Reference,
}

/// How a case's fixture comes into being.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Generation {
    /// Built from this manifest alone: `crates` says what to write.
    Declared,
    /// Written as Rust, because the seeded construct is Rust syntax.
    Authored,
}

/// The kind of a declared dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepKind {
    #[default]
    Normal,
    Dev,
    Build,
}

/// A dependency as the fixture should declare it. The four shapes C15 to C18
/// exercise — a target `cfg`, `optional`, a rename, and workspace inheritance —
/// are fields here rather than four hand-written Cargo.toml files.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dep {
    pub name: String,
    #[serde(default)]
    pub kind: DepKind,
    /// `"path"` or `"registry"`; defaults to `path` for a workspace member or
    /// an [`OutsideCrate`]. An outside crate's location is stated once, by
    /// [`OutsideCrate::at`], and the generator computes the relative path from
    /// there and the depending crate's own directory — cargo resolves a
    /// dependency path relative to the depending crate, not the workspace
    /// root, so a path repeated here would differ by one level for a member.
    pub source: Option<String>,
    pub version: Option<String>,
    /// A cfg string, written as `[target.'<cfg>'.dependencies]`.
    pub target: Option<String>,
    #[serde(default)]
    pub optional: bool,
    /// The key in `Cargo.toml` when it differs from the package name.
    pub rename: Option<String>,
    /// `true` writes `<name>.workspace = true`.
    #[serde(default)]
    pub inherit: bool,
}

/// A crate in a fixture workspace.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureCrate {
    pub name: String,
    /// The value written to `[package.metadata.rha] role`. `None` writes no
    /// metadata at all, which is what C10 needs.
    pub role: Option<String>,
    #[serde(default)]
    pub implements: Vec<String>,
    #[serde(default)]
    pub build_script: bool,
    #[serde(default)]
    pub deps: Vec<Dep>,
    /// Rust **items**, appended verbatim to the generated `src/lib.rs`. R01
    /// needs a private module in one crate and a reference to it from
    /// another; EM-C02 needs a call the crate graph cannot see.
    pub body: Option<String>,
}

/// A crate generated beside the fixture workspace rather than inside it (C19).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutsideCrate {
    pub name: String,
    /// Where the crate is generated, relative to the **workspace root**. The
    /// single owner of this crate's location.
    pub at: String,
    /// An outside crate may itself depend on another outside crate: EM-C01
    /// needs the transitive edge to exist without either end being a
    /// classified member of the fixture workspace.
    #[serde(default)]
    pub deps: Vec<Dep>,
}

/// A case's overrides to the generated rules file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub adapter_prefix: Option<String>,
    pub app_prefix: Option<String>,
    pub tools: Option<Vec<String>>,
    pub harness: Option<Vec<String>>,
    pub core_allow: Option<Vec<String>>,
    pub core_dev_allow: Option<Vec<String>>,
    pub core_allow_build_scripts: Option<bool>,
    pub adapters_require_port_owner_dependency: Option<bool>,
    pub transitive_enabled: Option<bool>,
    pub forbidden_edges: Option<Vec<BTreeMap<String, String>>>,
}

/// One pre-registered case.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub level: Level,
    pub generation: Generation,
    pub expected: Expected,
    /// The rule id a detection must carry, or `"-"` where no rule applies.
    pub rule: String,
    /// The §4.1 rows this case is evidence for, as keys of `[cells]`. A miss
    /// downgrades every one of them: one seeded violation can carry more
    /// than one claim.
    pub cells: Vec<String>,
    /// What the fixture seeds, in words.
    pub seeded: String,
    /// What a finding must name. **Every** key here is matched; prose belongs
    /// in [`Case::witness_notes`].
    #[serde(default)]
    pub witness: BTreeMap<String, toml::Value>,
    /// Prose for the reader, matched against nothing.
    #[serde(default)]
    pub witness_notes: BTreeMap<String, toml::Value>,
    /// Required when `expected` is `expected_miss`: the §4.1 hole that
    /// explains why the violation is out of reach.
    pub hole: Option<String>,
    /// Where the violation *is* caught, for a case one level does not reach.
    pub detected_when: Option<String>,
    /// The crates of a declared fixture workspace.
    #[serde(default)]
    pub crates: Vec<FixtureCrate>,
    #[serde(default)]
    pub outside_crates: Vec<OutsideCrate>,
    #[serde(default)]
    pub rules: Rules,
    /// A detector other than `cargo xtask architecture` (R01: `cargo check`).
    pub detector: Option<String>,
    /// Facts a detect case registers in advance, each matched by the same
    /// every-key rule as `witness` (DP-1.1c amendment, CHG-004.6). A finding
    /// matching one is neither the detection nor a false alarm, and never
    /// supplies a required detection. Only a detect case may register them.
    #[serde(default)]
    pub expected_findings: Vec<BTreeMap<String, toml::Value>>,
    /// The condition under which this case is re-registered, stated in advance.
    pub re_register_if: Option<String>,
    /// `"heuristic"` where the extraction is known to be approximate (M20).
    pub extraction: Option<String>,
    /// Which part of the case is expected to be missed (EM-M03).
    pub partial: Option<String>,
    pub depends_on: Option<String>,
    pub outcome_until_installed: Option<String>,
    pub reason_until_installed: Option<String>,
}

/// How a run turns observed findings into detected, missed and false alarm.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grading {
    pub proposed_by: String,
    /// Who decided the values, and where; `None` while they are a proposal.
    #[serde(default)]
    pub decided_by: Option<String>,
    #[serde(default)]
    pub decided_in: Option<String>,
    pub detection_requires: String,
    pub extra_findings: String,
    pub no_alarm_scope: String,
    pub miss_is_terminal: String,
    pub expected_miss_surprise: String,
}

/// The default rules file written into every crate-level fixture workspace.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesDefault {
    pub adapter_prefix: String,
    pub app_prefix: String,
    pub tools: Vec<String>,
    pub harness: Vec<String>,
    pub core_allow: Vec<String>,
    pub core_dev_allow: Vec<String>,
    pub core_allow_build_scripts: bool,
    pub adapters_require_port_owner_dependency: bool,
    pub transitive_enabled: bool,
    pub forbidden_edges: Vec<BTreeMap<String, String>>,
}

/// The manifest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub pre_registration: String,
    pub plan_ref: String,
    /// §4.1 enforcement-map rows, by id.
    pub cells: BTreeMap<String, String>,
    pub rules_default: RulesDefault,
    pub grading: Grading,
    #[serde(rename = "case")]
    pub cases: Vec<Case>,
}

/// Why a manifest is not usable as a pre-registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Defect {
    /// Two cases share an id, so a result cannot be attributed to one of them.
    DuplicateId(String),
    /// A case names a `[cells]` key that does not exist.
    UnknownCell { case: String, cell: String },
    /// A case names no §4.1 row, so a miss would downgrade nothing.
    NoCells(String),
    /// A case expects a detection but names no rule.
    DetectWithoutRule(String),
    /// An expected miss that cites no §4.1 hole is an excuse, not a record.
    MissWithoutHole(String),
    /// A declared case carries no workspace, so CHG-004 cannot generate it.
    DeclaredWithoutCrates(String),
    /// A declared case names a dependency on a crate that is neither in its
    /// workspace, outside it by path, nor an external registry crate.
    UnresolvableDep {
        case: String,
        krate: String,
        dep: String,
    },
    /// A case that must be detected states nothing a finding has to name.
    DetectWithoutWitness(String),
    /// A prose key sits in `witness`, where every key is matched. A harness
    /// would try to match a sentence against a finding's field.
    ProseInWitness { case: String, key: String },
    /// A registered fact on a case that is not a detect case: the DP-1.1c
    /// amendment covers detect cases only.
    RegisteredFactOutsideDetect(String),
    /// A registered fact with no keys would match every finding.
    EmptyRegisteredFact(String),
    /// A prose key in a registered fact, where every key is matched.
    ProseInRegisteredFact { case: String, key: String },
}

/// Keys that are prose, and therefore belong in `witness_notes`. The grading
/// rule matches every key of `witness`, so a sentence left there would be
/// compared against a finding's field.
pub const PROSE_KEYS: [&str; 3] = ["note", "alternative", "applies_when"];

impl std::fmt::Display for Defect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "case id {id} appears more than once"),
            Self::UnknownCell { case, cell } => {
                write!(
                    f,
                    "case {case} names cell {cell}, which [cells] does not define"
                )
            }
            Self::NoCells(id) => write!(
                f,
                "case {id} names no §4.1 cell, so a miss on it would downgrade nothing"
            ),
            Self::DetectWithoutRule(id) => {
                write!(f, "case {id} expects a detection but its rule is \"-\"")
            }
            Self::MissWithoutHole(id) => write!(
                f,
                "case {id} is an expected miss and cites no §4.1 hole; an expected miss without a hole is an excuse"
            ),
            Self::DeclaredWithoutCrates(id) => write!(
                f,
                "case {id} is declared but lists no crates, so CHG-004 cannot generate its fixture"
            ),
            Self::UnresolvableDep { case, krate, dep } => write!(
                f,
                "case {case}: {krate} depends on {dep}, which is not a member of the fixture workspace, not an outside crate, and not marked as a registry dependency"
            ),
            Self::DetectWithoutWitness(id) => write!(
                f,
                "case {id} expects a detection and states no witness, so any finding at all would score it"
            ),
            Self::ProseInWitness { case, key } => write!(
                f,
                "case {case} has the prose key {key} in witness, where every key is matched; it belongs in witness_notes"
            ),
            Self::RegisteredFactOutsideDetect(id) => write!(
                f,
                "case {id} registers an expected finding but is not a detect case; the DP-1.1c amendment covers detect cases only"
            ),
            Self::EmptyRegisteredFact(id) => write!(
                f,
                "case {id} registers an expected finding with no keys, which would match every finding"
            ),
            Self::ProseInRegisteredFact { case, key } => write!(
                f,
                "case {case} has the prose key {key} in a registered fact, where every key is matched"
            ),
        }
    }
}

impl Manifest {
    /// Parses a manifest. Unknown fields are rejected: a typo in a case must
    /// not read as a case with a missing field.
    ///
    /// # Errors
    /// Returns the TOML error when `text` is not a manifest.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Every defect in the manifest, in case order.
    #[must_use]
    pub fn defects(&self) -> Vec<Defect> {
        let mut defects = Vec::new();
        let mut seen = BTreeSet::new();
        for case in &self.cases {
            if !seen.insert(case.id.as_str()) {
                defects.push(Defect::DuplicateId(case.id.clone()));
            }
            if case.cells.is_empty() {
                defects.push(Defect::NoCells(case.id.clone()));
            }
            if !case.expected_findings.is_empty() && case.expected != Expected::Detect {
                defects.push(Defect::RegisteredFactOutsideDetect(case.id.clone()));
            }
            for entry in &case.expected_findings {
                if entry.is_empty() {
                    defects.push(Defect::EmptyRegisteredFact(case.id.clone()));
                }
                for key in entry.keys() {
                    if PROSE_KEYS.contains(&key.as_str()) {
                        defects.push(Defect::ProseInRegisteredFact {
                            case: case.id.clone(),
                            key: key.clone(),
                        });
                    }
                }
            }
            for cell in &case.cells {
                if !self.cells.contains_key(cell) {
                    defects.push(Defect::UnknownCell {
                        case: case.id.clone(),
                        cell: cell.clone(),
                    });
                }
            }
            match case.expected {
                Expected::Detect => {
                    if case.rule == "-" {
                        defects.push(Defect::DetectWithoutRule(case.id.clone()));
                    }
                    if case.witness.is_empty() {
                        defects.push(Defect::DetectWithoutWitness(case.id.clone()));
                    }
                }
                Expected::ExpectedMiss if case.hole.is_none() => {
                    defects.push(Defect::MissWithoutHole(case.id.clone()));
                }
                _ => {}
            }
            for key in case.witness.keys() {
                if PROSE_KEYS.contains(&key.as_str()) {
                    defects.push(Defect::ProseInWitness {
                        case: case.id.clone(),
                        key: key.clone(),
                    });
                }
            }
            if case.generation == Generation::Declared && case.crates.is_empty() {
                defects.push(Defect::DeclaredWithoutCrates(case.id.clone()));
            }
            defects.extend(Self::unresolvable_deps(case));
        }
        defects
    }

    /// Dependencies a generator could not resolve to a file it writes.
    fn unresolvable_deps(case: &Case) -> Vec<Defect> {
        let members: BTreeSet<&str> = case.crates.iter().map(|c| c.name.as_str()).collect();
        let outside: BTreeSet<&str> = case
            .outside_crates
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        let mut defects = Vec::new();
        let all: Vec<(&str, &Vec<Dep>)> = case
            .crates
            .iter()
            .map(|c| (c.name.as_str(), &c.deps))
            .chain(
                case.outside_crates
                    .iter()
                    .map(|c| (c.name.as_str(), &c.deps)),
            )
            .collect();
        for (krate_name, deps) in all {
            for dep in deps {
                let name = dep.name.as_str();
                let resolvable = members.contains(name)
                    || outside.contains(name)
                    || dep.source.as_deref() == Some("registry");
                if !resolvable {
                    defects.push(Defect::UnresolvableDep {
                        case: case.id.clone(),
                        krate: krate_name.to_owned(),
                        dep: dep.name.clone(),
                    });
                }
            }
        }
        defects
    }

    /// The ids of every case, in manifest order.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.cases.iter().map(|c| c.id.as_str()).collect()
    }

    /// How many cases carry each expected outcome at `level`.
    #[must_use]
    pub fn counts(&self, level: Level) -> BTreeMap<Expected, usize> {
        let mut counts = BTreeMap::new();
        for case in self.cases.iter().filter(|c| c.level == level) {
            *counts.entry(case.expected).or_insert(0) += 1;
        }
        counts
    }

    /// Every distinct rule id a case names, excluding the placeholder `"-"`.
    #[must_use]
    pub fn rules(&self) -> BTreeSet<&str> {
        self.cases
            .iter()
            .map(|c| c.rule.as_str())
            .filter(|r| *r != "-")
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest with one of everything the validation looks at.
    fn minimal() -> String {
        r#"
schema_version = 1
pre_registration = "test"
plan_ref = "test"
[cells]
"law5" = "Law 5: explicit effects"
[rules_default]
adapter_prefix = "adapter-"
app_prefix = "app-"
tools = []
harness = []
core_allow = []
core_dev_allow = []
core_allow_build_scripts = false
adapters_require_port_owner_dependency = true
transitive_enabled = false
forbidden_edges = []
[grading]
proposed_by = "x"
decided_by = "k"
decided_in = "t"
detection_requires = "a"
extra_findings = "b"
no_alarm_scope = "c"
miss_is_terminal = "d"
expected_miss_surprise = "e"
[[case]]
id = "C01"
level = "crate"
generation = "declared"
expected = "detect"
rule = "effect.core_build_script"
cells = ["law5"]
seeded = "a core with a build script"
witness = { rule = "effect.core_build_script", crate = "core-a" }
crates = [{ name = "core-a", role = "core", build_script = true }]
"#
        .to_owned()
    }

    #[test]
    fn a_well_formed_manifest_has_no_defects() {
        let manifest = Manifest::parse(&minimal()).unwrap();
        assert_eq!(manifest.defects(), vec![]);
        assert_eq!(manifest.ids(), vec!["C01"]);
        assert_eq!(manifest.counts(Level::Crate)[&Expected::Detect], 1);
        assert!(manifest.rules().contains("effect.core_build_script"));
    }

    #[test]
    fn a_repeated_id_is_a_defect() {
        let text = minimal().replace(
            "crates = [{ name = \"core-a\", role = \"core\", build_script = true }]",
            "crates = [{ name = \"core-a\", role = \"core\", build_script = true }]\n\
             [[case]]\nid = \"C01\"\nlevel = \"crate\"\ngeneration = \"declared\"\n\
             expected = \"detect\"\nrule = \"effect.core_build_script\"\ncells = [\"law5\"]\n\
             seeded = \"again\"\nwitness = { crate = \"core-a\" }\n\
             crates = [{ name = \"core-a\" }]",
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert!(
            manifest
                .defects()
                .contains(&Defect::DuplicateId("C01".to_owned()))
        );
    }

    #[test]
    fn an_unknown_cell_is_a_defect() {
        let text = minimal().replace(r#"cells = ["law5"]"#, r#"cells = ["law99"]"#);
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(
            manifest.defects(),
            vec![Defect::UnknownCell {
                case: "C01".to_owned(),
                cell: "law99".to_owned(),
            }]
        );
    }

    #[test]
    fn an_expected_miss_without_a_hole_is_a_defect() {
        let text = minimal().replace(r#"expected = "detect""#, r#"expected = "expected_miss""#);
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(
            manifest.defects(),
            vec![Defect::MissWithoutHole("C01".to_owned())]
        );
    }

    #[test]
    fn a_detection_without_a_rule_or_a_witness_is_a_defect() {
        let no_rule = minimal().replace(r#"rule = "effect.core_build_script""#, r#"rule = "-""#);
        assert!(
            Manifest::parse(&no_rule)
                .unwrap()
                .defects()
                .contains(&Defect::DetectWithoutRule("C01".to_owned()))
        );

        let no_witness = minimal().replace(
            r#"witness = { rule = "effect.core_build_script", crate = "core-a" }"#,
            "witness = {}",
        );
        assert!(
            Manifest::parse(&no_witness)
                .unwrap()
                .defects()
                .contains(&Defect::DetectWithoutWitness("C01".to_owned()))
        );
    }

    #[test]
    fn a_declared_case_without_crates_cannot_be_generated() {
        let text = minimal().replace(
            r#"crates = [{ name = "core-a", role = "core", build_script = true }]"#,
            "",
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert!(
            manifest
                .defects()
                .contains(&Defect::DeclaredWithoutCrates("C01".to_owned()))
        );
    }

    #[test]
    fn a_dependency_on_a_crate_the_generator_would_not_write_is_a_defect() {
        let text = minimal().replace(
            r#"crates = [{ name = "core-a", role = "core", build_script = true }]"#,
            r#"crates = [{ name = "core-a", role = "core", deps = [{ name = "ghost" }] }]"#,
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(
            manifest.defects(),
            vec![Defect::UnresolvableDep {
                case: "C01".to_owned(),
                krate: "core-a".to_owned(),
                dep: "ghost".to_owned(),
            }]
        );
    }

    /// An outside crate's own dependencies are checked like a member's. They
    /// were not until CHG-002.1, which is how EM-C01's `pure-looking` could
    /// name a crate the generator had no instructions to write.
    #[test]
    fn an_outside_crates_dependencies_are_validated_too() {
        let text = minimal().replace(
            r#"crates = [{ name = "core-a", role = "core", build_script = true }]"#,
            "crates = [{ name = \"core-a\", role = \"core\", deps = [{ name = \"friend\" }] }]\n             outside_crates = [{ name = \"friend\", at = \"../friend\", deps = [{ name = \"ghost\" }] }]",
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(
            manifest.defects(),
            vec![Defect::UnresolvableDep {
                case: "C01".to_owned(),
                krate: "friend".to_owned(),
                dep: "ghost".to_owned(),
            }]
        );
    }

    #[test]
    fn a_case_naming_no_cell_is_a_defect() {
        let text = minimal().replace("cells = [\"law5\"]\nseeded", "cells = []\nseeded");
        let manifest = Manifest::parse(&text).unwrap();
        assert!(
            manifest
                .defects()
                .contains(&Defect::NoCells("C01".to_owned()))
        );
    }

    #[test]
    fn a_prose_key_left_in_witness_is_a_defect() {
        let text = minimal().replace(
            r#"witness = { rule = "effect.core_build_script", crate = "core-a" }"#,
            r#"witness = { rule = "effect.core_build_script", crate = "core-a", note = "a sentence" }"#,
        );
        let manifest = Manifest::parse(&text).unwrap();
        assert_eq!(
            manifest.defects(),
            vec![Defect::ProseInWitness {
                case: "C01".to_owned(),
                key: "note".to_owned(),
            }]
        );
    }

    #[test]
    fn an_unknown_field_is_rejected_rather_than_ignored() {
        let text = minimal().replace(
            r#"seeded = "a core with a build script""#,
            "seeded = \"x\"\nexpceted = \"detect\"",
        );
        assert!(Manifest::parse(&text).is_err());
    }
}
