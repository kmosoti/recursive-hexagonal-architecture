#![allow(dead_code, unused_imports)]
pub mod constraints {
pub struct Rule;
pub struct X; impl crate::ordering::Trait for X {}
}
pub mod ordering {
pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }
pub fn reverse(_: crate::constraints::Rule) {}
}
