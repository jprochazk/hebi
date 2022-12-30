use beef::lean::Cow;
use span::Span;

pub mod ast;
pub mod lexer;
pub mod lexer2;
pub mod parser;
pub mod parser2;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct Error {
  pub message: Cow<'static, str>,
  pub span: Span,
}

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>, span: impl Into<Span>) -> Self {
    let message = message.into();
    let span = span.into();
    Error { message, span }
  }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Error { message, span } = self;
    write!(f, "error at {span}: {message}")
  }
}
