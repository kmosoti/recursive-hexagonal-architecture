#![allow(dead_code, unused_imports)]
pub mod constraints {
pub struct Rule;
pub fn f() { assert_eq!(crate::ordering::score(), 1); }
}
pub mod ordering {
pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }
pub fn reverse(_: crate::constraints::Rule) {}
}
