//! Total classification of every workspace member (plan §5, spec §11.2).
//!
//! Precedence, in order: an explicit `[package.metadata.rha] role` wins; else
//! the `adapter-` or `app-` prefix; else the `tools` or `harness` list; else
//! nothing classified it and that is `class.unclassified`, an error.
//!
//! "Total" is the point. A member that matches no rule does not fall through
//! to a default role, because a default would let an unclassified crate pass
//! every direction rule silently. It is reported instead (corpus C10).
//!
//! The prefix is matched as a **prefix**, with `starts_with`, never as a
//! substring. Corpus case L11 declares `planner-adapter-utils` with
//! `role = "core"`: the name contains `adapter-` and does not begin with it,
//! so there is no conflict to report, and a checker that searches anywhere in
//! the name fails that case.

use crate::graph::model::{CrateNode, Role, RoleSource};
use crate::graph::rules::Classification;

/// Why a crate could not be classified, or was classified inconsistently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Nothing classified the crate (`class.unclassified`).
    Unclassified,
    /// A prefix and a declared role disagree (`class.prefix_role_conflict`).
    PrefixRoleConflict {
        prefix_says: Role,
        metadata_says: Role,
    },
    /// `[package.metadata.rha] role` is not one of the five roles.
    UnknownRole { declared: String },
}

/// The outcome of classifying one crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classified {
    pub role: Option<Role>,
    pub source: Option<RoleSource>,
    pub problem: Option<Problem>,
}

/// The role a prefix implies, if any.
fn by_prefix(name: &str, classification: &Classification) -> Option<Role> {
    if !classification.adapter_prefix.is_empty() && name.starts_with(&classification.adapter_prefix)
    {
        return Some(Role::Adapter);
    }
    if !classification.app_prefix.is_empty() && name.starts_with(&classification.app_prefix) {
        return Some(Role::App);
    }
    None
}

/// The role a list implies, if any.
fn by_list(name: &str, classification: &Classification) -> Option<Role> {
    if classification.tools.iter().any(|t| t == name) {
        return Some(Role::Tool);
    }
    if classification.harness.iter().any(|h| h == name) {
        return Some(Role::Harness);
    }
    None
}

/// Classifies one crate.
#[must_use]
pub fn classify(node: &CrateNode, classification: &Classification) -> Classified {
    let prefix = by_prefix(&node.name, classification);

    if let Some(declared) = &node.declared_role {
        let Some(role) = Role::parse(declared) else {
            return Classified {
                role: None,
                source: None,
                problem: Some(Problem::UnknownRole {
                    declared: declared.clone(),
                }),
            };
        };
        // Metadata wins, and the crate IS classified — but a disagreement
        // with the prefix is still reported, because one of the two is wrong
        // and a reader cannot tell which from the crate alone (corpus C11).
        let problem = prefix
            .filter(|p| *p != role)
            .map(|p| Problem::PrefixRoleConflict {
                prefix_says: p,
                metadata_says: role,
            });
        return Classified {
            role: Some(role),
            source: Some(RoleSource::Metadata),
            problem,
        };
    }

    if let Some(role) = prefix {
        return Classified {
            role: Some(role),
            source: Some(RoleSource::Prefix),
            problem: None,
        };
    }
    if let Some(role) = by_list(&node.name, classification) {
        return Classified {
            role: Some(role),
            source: Some(RoleSource::List),
            problem: None,
        };
    }
    Classified {
        role: None,
        source: None,
        problem: Some(Problem::Unclassified),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn rules() -> Classification {
        Classification {
            adapter_prefix: "adapter-".to_owned(),
            app_prefix: "app-".to_owned(),
            tools: vec!["xtask".to_owned()],
            harness: vec!["harness-h".to_owned()],
        }
    }

    fn node(name: &str, declared_role: Option<&str>) -> CrateNode {
        CrateNode {
            name: name.to_owned(),
            manifest_path: PathBuf::from(format!("/w/{name}/Cargo.toml")),
            role: None,
            role_source: None,
            declared_role: declared_role.map(ToOwned::to_owned),
            implements: Vec::new(),
            has_build_script: false,
        }
    }

    #[test]
    fn metadata_wins_over_a_prefix_and_the_disagreement_is_still_reported() {
        // Corpus C11.
        let got = classify(&node("adapter-x", Some("core")), &rules());
        assert_eq!(
            (got.role, got.source),
            (Some(Role::Core), Some(RoleSource::Metadata))
        );
        assert_eq!(
            got.problem,
            Some(Problem::PrefixRoleConflict {
                prefix_says: Role::Adapter,
                metadata_says: Role::Core,
            })
        );
    }

    #[test]
    fn metadata_agreeing_with_the_prefix_is_not_a_conflict() {
        let got = classify(&node("adapter-x", Some("adapter")), &rules());
        assert_eq!(got.role, Some(Role::Adapter));
        assert_eq!(got.problem, None);
    }

    #[test]
    fn the_prefix_is_a_prefix_and_not_a_substring() {
        // Corpus L11: the name contains "adapter-" without starting with it,
        // so the prefix does not fire and there is nothing to conflict with.
        let got = classify(&node("planner-adapter-utils", Some("core")), &rules());
        assert_eq!(got.role, Some(Role::Core));
        assert_eq!(
            got.problem, None,
            "a substring match would report a conflict here"
        );

        // And with no metadata at all it is simply unclassified, not an adapter.
        let bare = classify(&node("planner-adapter-utils", None), &rules());
        assert_eq!(bare.role, None);
        assert_eq!(bare.problem, Some(Problem::Unclassified));
    }

    #[test]
    fn a_prefix_classifies_when_no_metadata_is_declared() {
        for (name, role) in [("adapter-x", Role::Adapter), ("app-main", Role::App)] {
            let got = classify(&node(name, None), &rules());
            assert_eq!(
                (got.role, got.source),
                (Some(role), Some(RoleSource::Prefix))
            );
            assert_eq!(got.problem, None);
        }
    }

    #[test]
    fn a_list_classifies_when_no_prefix_matches() {
        for (name, role) in [("xtask", Role::Tool), ("harness-h", Role::Harness)] {
            let got = classify(&node(name, None), &rules());
            assert_eq!((got.role, got.source), (Some(role), Some(RoleSource::List)));
        }
    }

    #[test]
    fn a_member_that_nothing_classifies_is_reported_not_defaulted() {
        // Corpus C10. A default role here would let this crate pass every
        // direction rule without anyone deciding what it is.
        let got = classify(&node("mystery", None), &rules());
        assert_eq!((got.role, got.source), (None, None));
        assert_eq!(got.problem, Some(Problem::Unclassified));
    }

    #[test]
    fn a_role_outside_the_five_does_not_classify() {
        let got = classify(&node("odd", Some("middleware")), &rules());
        assert_eq!(got.role, None);
        assert_eq!(
            got.problem,
            Some(Problem::UnknownRole {
                declared: "middleware".to_owned()
            })
        );
    }

    #[test]
    fn an_empty_prefix_matches_nothing_rather_than_everything() {
        // `starts_with("")` is true for every string, so an empty prefix
        // would classify the whole workspace as adapters.
        let empty = Classification {
            adapter_prefix: String::new(),
            app_prefix: String::new(),
            tools: Vec::new(),
            harness: Vec::new(),
        };
        assert_eq!(classify(&node("anything", None), &empty).role, None);
    }
}
