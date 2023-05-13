// TODO: get rid of this somehow
// it's only used in `value/object.rs` to access the vtable of a `dyn Object`
#![feature(ptr_metadata)]

use std::error::Error as StdError;
use std::fmt::Display;

use span::SpannedError;
use syntax::SyntaxError;

#[macro_use]
mod util;

#[macro_export]
macro_rules! fail {
  ($span:expr, $fmt:literal $(,$($arg:tt)*)?) => {
    return Err($crate::span::SpannedError::new(format!($fmt $(, $($arg)*)?), $span).into())
  };
  ($span:expr, $msg:expr) => {
    return Err($crate::span::SpannedError::new($msg, $span).into())
  };
}

pub mod bytecode;
pub mod ctx;
pub mod emit;
pub mod object;
pub mod span;
pub mod syntax;
pub mod value;
pub mod vm;

pub type HebiResult<T, E = HebiError> = core::result::Result<T, E>;

#[derive(Debug)]
pub enum HebiError {
  Vm(SpannedError),
  Syntax(SyntaxError),
  User(Box<dyn StdError + 'static>),
}

impl From<SpannedError> for HebiError {
  fn from(value: SpannedError) -> Self {
    HebiError::Vm(value)
  }
}

impl From<SyntaxError> for HebiError {
  fn from(value: SyntaxError) -> Self {
    HebiError::Syntax(value)
  }
}

impl Display for HebiError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      HebiError::Vm(e) => {
        write!(f, "{e}")
      }
      HebiError::Syntax(e) => {
        write!(f, "{e}")
      }
      HebiError::User(e) => {
        write!(f, "{e}")
      }
    }
  }
}

impl StdError for HebiError {}
