use beef::lean::Cow;

pub struct Error(pub value::object::Error);

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
    Self(value::object::Error::new(message))
  }

  pub fn add_trace(&mut self, s: String) {
    self.0.add_trace(s)
  }

  pub fn traceback<'a>(&self, source: impl Into<diag::Source<'a>>) -> String {
    use std::fmt::Write;

    // TODO: use trace
    let mut report = diag::Report::error()
      .source(source)
      .message(self.0.message.clone());

    if let Some(span) = self.0.span {
      report = report.span(span);
    }

    let mut trace = String::new();

    for s in self.0.trace.iter().rev() {
      writeln!(&mut trace, "in {s}").unwrap();
    }
    write!(&mut trace, "{}", report.build().emit_to_string().unwrap()).unwrap();

    trace
  }
}
