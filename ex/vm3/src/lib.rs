#![no_std]
extern crate alloc;

#[macro_use]
pub mod util;

pub mod ast;
pub mod gc;
pub mod lex;
pub mod obj;
pub mod op;
pub mod syn;
pub mod val;

type Arena = bumpalo::Bump;

pub use beef::lean::Cow;
