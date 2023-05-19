// TODO: get rid of this somehow
// it's only used in `value/object.rs` to access the vtable of a `dyn Object`
#![feature(ptr_metadata)]

#[macro_use]
pub mod macros;

#[macro_use]
mod util;

mod bytecode;
mod emit;
mod error;
mod object;
pub mod span;
mod syntax;
mod value;
mod vm;

pub mod public;
pub use public::*;
