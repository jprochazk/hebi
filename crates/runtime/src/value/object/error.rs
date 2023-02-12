use beef::lean::Cow;
use span::Span;

#[derive(Clone, Debug)]
pub struct Error {
  pub message: Cow<'static, str>,
  pub span: Option<Span>,
  pub trace: Vec<String>,
}

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
    Self {
      message: message.into(),
      span: None,
      trace: vec![],
    }
  }

  pub fn add_trace(&mut self, s: String) {
    self.trace.push(s);
  }
}
