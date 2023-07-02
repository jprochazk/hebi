#![allow(dead_code)] // TEMP

#[macro_use]
pub mod macros;

#[macro_use]
mod util;

mod internal {
  #[macro_use]
  pub(crate) mod object;

  pub(crate) mod bytecode;
  pub(crate) mod codegen;
  #[cfg(feature = "serde")]
  pub(crate) mod serde;
  pub(crate) mod syntax;
  pub(crate) mod value;
  pub(crate) mod vm;

  pub mod error;
}

pub mod public;
#[cfg(feature = "serde")]
pub mod serde;
pub mod span;

pub mod prelude {
  pub use super::public::*;
  #[cfg(feature = "serde")]
  pub use super::serde::ValueDeserializer;
}

pub use internal::error::{Error, Result};
