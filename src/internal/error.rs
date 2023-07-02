use std::error::Error as StdError;
use std::fmt::Display;

use super::syntax::SyntaxError;
use crate::span::SpannedError;

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
  Vm(SpannedError),
  Syntax(SyntaxError),
  User(Box<dyn StdError + Send + Sync + 'static>),
}

impl Error {
  pub fn user(e: impl StdError + Send + Sync + 'static) -> Self {
    Self::User(Box::new(e))
  }

  pub fn report(&self, src: &str, use_color: bool) -> String {
    match self {
      Error::Vm(e) => format!("runtime error: {}", e.report(src, use_color)),
      Error::Syntax(e) => {
        use std::fmt::Write;
        let mut s = "syntax error:\n".to_string();
        for e in e.errors() {
          writeln!(&mut s, "{}", e.report(src, use_color)).unwrap();
        }
        s
      }
      Error::User(e) => {
        // TODO: spans in user errors
        format!("runtime error: {e}")
      }
    }
  }
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
