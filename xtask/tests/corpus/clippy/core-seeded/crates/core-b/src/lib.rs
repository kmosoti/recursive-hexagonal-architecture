//! Fixture: a core crate with the template and nothing on the deny list. Its
//! test calls `unwrap()`, which the template's repeated allowance permits.

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
