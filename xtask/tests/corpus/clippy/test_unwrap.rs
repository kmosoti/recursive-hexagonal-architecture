//! Clippy corpus source (ADR-0002): nothing on the deny list; its test calls
//! `unwrap()`, which only a configuration with `allow-unwrap-in-tests` permits.

/// Adds one.
#[must_use]
pub fn next(n: u32) -> u32 {
    n + 1
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses() {
        let n: u32 = "41".parse().unwrap();
        assert_eq!(super::next(n), 42);
    }
}
