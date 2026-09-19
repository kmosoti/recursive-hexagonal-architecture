//! Clippy corpus source (ADR-0002): the seeded call with an item-level
//! `#[expect]`, the other form of the attribute escape hatch.

use std::time::SystemTime;

/// Reads the clock, with the finding declared expected for this item.
#[expect(clippy::disallowed_methods)]
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}
