use beef::lean::Cow;
use span::Span;

#[derive(Clone, Debug)]
pub struct Error {
  pub message: Cow<'static, str>,
  pub span: Span,
}

// TODO: spans + reports

#[allow(dead_code)]
impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>, span: impl Into<Span>) -> Self {
    let message = message.into();
    let span = span.into();
    Error { message, span }
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

  pub fn report_to<'a, W: ?Sized + std::fmt::Write>(
    &self,
    source: impl Into<diag::Source<'a>>,
    w: &mut W,
  ) {
    diag::Report::error()
      .source(source)
      .message(format!("{self}"))
      .span(self.span)
      .build()
      .emit(w)
      .unwrap()
  }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}
