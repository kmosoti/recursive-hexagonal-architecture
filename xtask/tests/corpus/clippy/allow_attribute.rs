//! Clippy corpus source (ADR-0002): the seeded call with an item-level
//! `#[allow]`, the escape hatch Clippy's own help text suggests.

use std::time::SystemTime;

/// Reads the clock, with the deny list switched off for this item.
#[allow(clippy::disallowed_methods)]
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}
