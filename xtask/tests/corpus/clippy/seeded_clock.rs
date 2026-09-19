//! Clippy corpus source (ADR-0002): reads the clock directly, the seeded
//! ambient effect. Generated crates pair it with different configurations.

use std::time::SystemTime;

/// The seeded ambient effect.
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}
