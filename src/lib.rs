// TODO: get rid of this somehow
// it's only used in `value/object.rs` to access the vtable of a `dyn Object`
#![feature(ptr_metadata)]

use std::error::Error as StdError;
use std::fmt::Display;

use span::SpannedError;
use syntax::SyntaxError;

#[macro_use]
pub mod macros;

#[macro_use]
mod util;

pub mod bytecode;
pub mod ctx;
pub mod emit;
pub mod object;
pub mod span;
pub mod syntax;
pub mod value;
pub mod vm;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
  Vm(SpannedError),
  Syntax(SyntaxError),
  User(Box<dyn StdError + 'static>),
}

impl From<SpannedError> for Error {
  fn from(value: SpannedError) -> Self {
    Error::Vm(value)
  }
}

impl From<SyntaxError> for Error {
  fn from(value: SyntaxError) -> Self {
    Error::Syntax(value)
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Error::Vm(e) => {
        write!(f, "{e}")
      }
      Error::Syntax(e) => {
        write!(f, "{e}")
      }
      Error::User(e) => {
        write!(f, "{e}")
      }
    }
  }
}

impl StdError for Error {}
