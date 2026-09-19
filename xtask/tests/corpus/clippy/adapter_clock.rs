//! Clippy corpus source (ADR-0002): an adapter that makes the seeded call, as
//! an adapter may, and whose test calls `unwrap()`.

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
