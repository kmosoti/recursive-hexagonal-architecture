#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]
pub fn host() -> std::io::Result<Vec<u8>> { std::fs::read("/etc/hostname") }
