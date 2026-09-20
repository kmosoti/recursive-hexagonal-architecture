//! The crate-graph checker (plan §5; spec §6.13).
//!
//! `model` is the graph, `classify` decides roles, `rules` is the rules file,
//! `check` evaluates the rules, and `report` renders the result. Nothing here
//! reads `xtask/tests/corpus/manifest.toml`: the corpus is the input to the
//! H4 harness of CHG-004, and a checker that reads the cases it is judged on
//! can satisfy one without implementing the rule.

pub mod check;
pub mod classify;
pub mod model;
pub mod report;
pub mod rules;
