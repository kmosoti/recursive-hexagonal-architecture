//! The acceptance model of spec §11.7.6, over the fixture format fixed in
//! `docs/architecture/verifier-contract.md` before this code existed (P-B).
//!
//! [`evaluate`] takes a fixture (its `expected` member is ignored) and
//! returns the decision: `r_eff`, `conflict`, the four predicates,
//! `eligible`, `valid_exception` and `merge_allowed`.

mod glob;
mod model;
mod time;

pub use glob::glob;
pub use model::evaluate;
pub use time::parse_rfc3339;
