use std::fmt::{Debug, Display};

use beef::lean::Cow;
use diag::Source;
use span::Span;

use super::Access;

pub(crate) enum Cause<'a> {
  Script(Cow<'a, str>),
  Native(Box<dyn std::error::Error + 'a>),
}

impl<'a> Debug for Cause<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Script(v) => Debug::fmt(v, f),
      Self::Native(v) => Debug::fmt(v, f),
    }
  }
}

impl<'a> Display for Cause<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Cause::Script(v) => Display::fmt(v, f),
      Cause::Native(v) => Display::fmt(v, f),
    }
  }
}

pub struct RuntimeError {
  cause: Cause<'static>,
  span: Span,
  trace: Vec<Fragment>,
}

#[derive(Clone, Debug)]
pub struct Fragment {
  pub ident: String,
  pub span: Span,
  pub file: Option<String>,
}

impl RuntimeError {
  pub(crate) fn script(message: impl Into<Cow<'static, str>>, span: impl Into<Span>) -> Self {
    Self {
      cause: Cause::Script(message.into()),
      span: span.into(),
      trace: vec![],
    }
  }

  pub(crate) fn native(error: Box<dyn std::error::Error + 'static>, span: impl Into<Span>) -> Self {
    Self {
      cause: Cause::Native(error),
      span: span.into(),
      trace: vec![],
    }
  }
}

#[derive::delegate_to_handle]
impl RuntimeError {
  pub(crate) fn push_trace(
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
    writeln!(to, "Error: {}", self.cause).unwrap();
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

impl Display for RuntimeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<error>")
  }
}

impl Debug for RuntimeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.cause {
      Cause::Script(e) => f.debug_tuple("Script").field(&e).finish(),
      Cause::Native(e) => f.debug_tuple("Native").field(&e).finish(),
    }
  }
}

impl Access for RuntimeError {}
