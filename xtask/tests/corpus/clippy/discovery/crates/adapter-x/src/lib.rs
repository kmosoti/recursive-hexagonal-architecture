//! Fixture: an adapter crate with no local `clippy.toml`. It makes the call
//! that is seeded in core-seeded's core-a, and its test calls `unwrap()`.

use std::time::SystemTime;

/// Reads the system clock, as an adapter may.
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses() {
        let n: u32 = "42".parse().unwrap();
        assert_eq!(n, 42);
        let _ = super::now();
    }
}
