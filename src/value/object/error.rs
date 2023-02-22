use std::fmt::Display;

use beef::lean::Cow;
use diag::Source;
use span::Span;

use super::Access;

#[derive(Clone, Debug)]
pub struct Error {
  pub message: Cow<'static, str>,
  pub span: Span,
  pub trace: Vec<Fragment>,
}

#[derive(Clone, Debug)]
pub struct Fragment {
  pub ident: String,
  pub span: Span,
  pub file: Option<String>,
}

impl Error {
  pub fn new(message: impl Into<Cow<'static, str>>, span: impl Into<Span>) -> Self {
    Self {
      message: message.into(),
      span: span.into(),
      trace: vec![],
    }
  }
}

#[derive::delegate_to_handle]
impl Error {
  pub fn push_trace(
    &mut self,
    ident: impl Into<String>,
    span: impl Into<Span>,
    file: Option<String>,
  ) {
    self.trace.push(Fragment {
      ident: ident.into(),
      span: span.into(),
      file,
    });
  }

  pub fn write_stack_trace<W: std::fmt::Write>(&self, to: &mut W, source: Option<Source>) {
    for Fragment { ident, span, file } in self.trace.iter().rev() {
      match file {
        Some(file) => writeln!(to, "File `{file}` in {ident} at {span}").unwrap(),
        None => writeln!(to, "In {ident} at {span}").unwrap(),
      }
      // TODO: some kind of file database
      // write_source_lines(to, &source, span);
    }
    writeln!(to, "Error: {}", self.message).unwrap();
    write_source_lines(to, &source, &self.span);
  }

  pub fn stack_trace(&self, source: Option<Source>) -> String {
    let mut s = String::new();
    self.write_stack_trace(&mut s, source);
    s
  }
}

fn write_source_lines<W: std::fmt::Write>(to: &mut W, source: &Option<Source>, span: &Span) {
  if span.is_empty() {
    return;
  }
  let Some(source) = &source else {
    return
  };
  let slice = &source.str()[span.range()];
  for line in slice.trim().split('\n') {
    writeln!(to, "  {line}").unwrap();
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<error>")
  }
}

impl Access for Error {}
