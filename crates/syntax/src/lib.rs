use beef::lean::Cow;
use span::Span;

pub mod ast;
pub mod lexer;
pub mod parser;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct Error {
  pub context: Option<Cow<'static, str>>,
  pub message: Cow<'static, str>,
  pub span: Span,
}

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>, span: impl Into<Span>) -> Self {
    let message = message.into();
    let span = span.into();
    Error {
      context: None,
      message,
      span,
    }
  }

  pub fn report<'a>(&self, source: impl Into<diag::Source<'a>>) -> String {
    diag::Report::error()
      .source(source)
      .message(format!("{self}"))
      .span(self.span)
      .build()
      .emit_to_string()
      .unwrap()
  }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(context) = &self.context {
      write!(f, "{} while parsing {}", self.message, context)
    } else {
      write!(f, "{}", self.message)
    }
  }
}

pub trait ErrorContext {
  /// Updates potential error to `{message} while parsing {context}`,
  /// e.g. `expected identifier while parsing field key`.
  ///
  /// Note that an error can only have one context message,
  /// so this should only be called on parsing primitives.
  /// such as `Parser::expect` and `Parser::ident`.
  fn context(self, message: impl Into<Cow<'static, str>>) -> Self;
}

impl<T> ErrorContext for Result<T, Error> {
  fn context(mut self, message: impl Into<Cow<'static, str>>) -> Self {
    if let Err(e) = self.as_mut() {
      e.context = Some(message.into())
    }
    self
  }
}
