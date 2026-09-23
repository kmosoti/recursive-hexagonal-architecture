#![allow(dead_code, unused_imports)]
pub mod constraints {
pub struct Rule;
}
pub mod ordering {
pub struct Wave; pub trait Trait {} pub fn score() -> u8 { 1 }
pub mod inner { pub struct Local; pub fn f(_: self::Local, _: super::Wave) {} }
pub fn f(_: self::inner::Local) {}
}
