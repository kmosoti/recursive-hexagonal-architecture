//! Repository automation for the RHA implementation program.
//!
//! `cargo xtask ci` runs a lane defined in `.rha/policy.toml` and writes an
//! evidence record; `cargo xtask docs` writes generated projections of the
//! records. This crate has role `tool`: nothing may depend on it.

pub mod architecture;
pub mod cli;
pub mod clippy_template;
pub mod corpus;
pub mod docs;
pub mod error;
pub mod evidence;
pub mod graph;
pub mod lanes;
pub mod metadata;
pub mod policy;
pub mod record_lint;
pub mod tools;
pub mod util;
