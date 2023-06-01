#![allow(dead_code)] // TEMP

#[macro_use]
pub mod macros;

#[macro_use]
mod util;

#[macro_use]
mod object;

mod bytecode;
mod codegen;
mod error;
#[cfg(feature = "serde")]
mod serde;
pub mod span;
mod syntax;
mod value;
mod vm;

pub mod public;
pub use public::*;
