#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(error_in_core))]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
pub(crate) use std as alloc;

#[macro_use]
pub mod util;

pub mod ast;
pub mod error;
pub mod gc;
pub mod lex;
pub mod obj;
pub mod op;
pub mod syn;
pub mod val;

type Arena = bumpalo::Bump;

pub use beef::lean::Cow;
