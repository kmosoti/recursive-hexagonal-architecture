#![allow(dead_code, unused_imports)]
pub mod model { pub struct Value; }
pub mod constraints {
pub struct Rule;
use crate::ordering::*; pub fn f() { let value = make(); value.touch(); }
}
pub mod ordering {
pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }
pub trait Hidden { fn touch(&self); }
impl Hidden for crate::model::Value { fn touch(&self) {} }
pub fn make() -> crate::model::Value { crate::model::Value }
}
