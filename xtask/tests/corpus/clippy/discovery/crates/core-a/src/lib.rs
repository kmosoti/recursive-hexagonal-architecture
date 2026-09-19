//! Fixture: a core crate whose local `clippy.toml` holds only the deny list.
//! Nothing here is on the deny list; its test calls `unwrap()`.

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
