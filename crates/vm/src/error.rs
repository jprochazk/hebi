use beef::lean::Cow;

pub struct Error(pub value::object::Error);

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
    Self(value::object::Error::new(message))
  }

  pub fn report<'a>(&self, source: impl Into<diag::Source<'a>>) -> String {
    // TODO: use trace
    let mut report = diag::Report::error()
      .source(source)
      .message(self.0.message.clone());

    if let Some(span) = self.0.span {
      report = report.span(span);
    }

    report.build().emit_to_string().unwrap()
  }
}
