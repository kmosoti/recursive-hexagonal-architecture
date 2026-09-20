#![forbid(clippy::disallowed_methods, clippy::disallowed_types, clippy::disallowed_macros)]
#[allow(unused_imports)] use core_a as _;
pub fn take(_: core_a::private_mod::Item) {}
