use std::error::Error as StdError;
use std::fmt::Display;

use crate::span::SpannedError;
use crate::syntax::SyntaxError;

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
