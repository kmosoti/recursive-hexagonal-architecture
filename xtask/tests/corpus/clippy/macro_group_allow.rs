//! Clippy corpus source (ADR-0002): an item carrying a group allow that a
//! macro expanded, the shape `clap`'s derives produce. Nothing here is an
//! ambient effect; the question is only what `forbid` does to the attribute.

macro_rules! quiet {
    ($name:ident) => {
        #[allow(clippy::style)]
        pub fn $name() -> u32 {
            1
        }
    };
}

quiet!(one);

/// Ordinary code beside the expansion.
#[must_use]
pub fn two() -> u32 {
    one() + 1
}
