//! Fixture: a core crate that reads the clock directly. Its `clippy.toml` is
//! the core template, whose deny list forbids the call. Its test calls
//! `unwrap()`, which the template's repeated test allowance permits.

use std::time::SystemTime;

/// The seeded ambient effect.
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses() {
        let n: u32 = "41".parse().unwrap();
        assert_eq!(n + 1, 42);
    }
}
