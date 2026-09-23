#![allow(dead_code, unused_imports)]
pub fn glue(_: crate::constraints::Rule, _: crate::ordering::Wave) {}
pub mod constraints {
pub struct Rule;
}
pub mod ordering {
pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }
}
