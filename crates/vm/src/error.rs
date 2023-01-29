use beef::lean::Cow;
use span::Span;

pub struct Error {
  pub message: Cow<'static, str>,
  pub span: Option<Span>,
}

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
    Self {
      message: message.into(),
      span: None,
    }
  }

  pub fn report<'a>(&self, source: impl Into<diag::Source<'a>>) -> String {
    let mut report = diag::Report::error()
      .source(source)
      .message(self.message.clone());

    if let Some(span) = self.span {
      report = report.span(span);
    }

    report.build().emit_to_string().unwrap()
  }
}
