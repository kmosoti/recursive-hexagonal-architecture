//! Clippy corpus source (ADR-0002): the seeded call under a crate-level
//! `#![allow]`, which switches the deny list off for the whole crate.

#![allow(clippy::disallowed_methods)]

use std::time::SystemTime;

/// Reads the clock in a crate that allows the lint.
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}
