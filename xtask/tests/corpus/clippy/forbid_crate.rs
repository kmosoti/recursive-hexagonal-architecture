//! Clippy corpus source (ADR-0002): a crate-level `#![forbid]` with an
//! item-level `#[allow]` below it, the candidate fix for the escape hatch.

#![forbid(clippy::disallowed_methods)]

use std::time::SystemTime;

/// Reads the clock and tries to allow the lint the crate forbids.
#[allow(clippy::disallowed_methods)]
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}
