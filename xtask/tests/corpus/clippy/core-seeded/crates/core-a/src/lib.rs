//! Fixture: a core crate that reads the clock directly. Its `clippy.toml` is
//! the core template, whose deny list forbids the call.

use std::time::SystemTime;

/// The seeded ambient effect.
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}
