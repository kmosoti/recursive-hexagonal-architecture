#![allow(dead_code, unused_imports)]
macro_rules! hidden { () => { crate::ordering::score() }; }
pub mod constraints {
pub struct Rule;
pub fn f() { let _ = hidden!(); }
}
pub mod ordering {
pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }
pub fn reverse(_: crate::constraints::Rule) {}
}
