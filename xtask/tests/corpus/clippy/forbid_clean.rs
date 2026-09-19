//! Clippy corpus source (ADR-0002): a core crate that closes the attribute
//! escape itself, with nothing on the deny list. The crate-root `forbid` is
//! what a core crate carries once CHG-005 creates them.

#![forbid(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::disallowed_macros
)]

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
