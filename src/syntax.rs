pub mod ast;
pub mod lexer;
pub mod parser;

use std::error::Error as StdError;
use std::fmt::Display;

pub use ast::Module;
pub use parser::parse;

use crate::span::SpannedError;
use crate::util::JoinIter;

#[derive(Debug)]
pub struct SyntaxError {
  errors: Vec<SpannedError>,
}

impl SyntaxError {
  fn new(errors: Vec<SpannedError>) -> Self {
    Self { errors }
  }

  pub fn errors(&self) -> &[SpannedError] {
    &self.errors
  }
}

impl Display for SyntaxError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.errors.iter().join("\n"))
  }
}

impl StdError for SyntaxError {}
