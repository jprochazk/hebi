use std::error::Error as StdError;
use std::fmt::{Debug, Display};

pub enum Error {
  Other(Box<dyn StdError + 'static>),
  Syntax(Vec<syntax::Error>),
  Runtime(String),
}

impl Error {
  pub fn other(e: Box<dyn StdError + 'static>) -> Self {
    Self::Other(e)
  }

  pub fn syntax(e: Vec<syntax::Error>) -> Self {
    Self::Syntax(e)
  }

  pub fn runtime(message: impl ToString) -> Self {
    Self::Runtime(message.to_string())
  }
}

impl From<Box<dyn StdError + 'static>> for Error {
  fn from(value: Box<dyn StdError + 'static>) -> Self {
    Self::other(value)
  }
}

impl From<Vec<syntax::Error>> for Error {
  fn from(value: Vec<syntax::Error>) -> Self {
    Self::syntax(value)
  }
}

impl From<String> for Error {
  fn from(value: String) -> Self {
    Self::runtime(value)
  }
}

impl crate::value::object::Access for Error {}
impl StdError for Error {}
impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self {
      Error::Other(e) => write!(f, "{e}"),
      Error::Syntax(e) => {
        for error in e.iter() {
          writeln!(f, "syntax error: {error}")?;
        }
        Ok(())
      }
      Error::Runtime(e) => write!(f, "error: {e}"),
    }
  }
}
impl Debug for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self {
      Error::Other(e) => f.debug_tuple("Error").field(&e).finish(),
      Error::Syntax(e) => f.debug_tuple("Error").field(&e).finish(),
      Error::Runtime(e) => f.debug_tuple("Error").field(&e).finish(),
    }
  }
}
