use std::fmt::{Debug, Display};
use std::sync::Arc;

use crate::lex::Span;

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

#[derive(Clone)]
pub struct Error {
  kind: ErrorKind,
  module_name: Arc<str>,
  source: Arc<str>,
  message: Arc<str>,
  span: Span,
}

impl Error {
  pub fn new(
    kind: ErrorKind,
    module_name: impl Into<Arc<str>>,
    source: impl Into<Arc<str>>,
    message: impl Into<Arc<str>>,
    span: impl Into<Span>,
  ) -> Self {
    Self {
      kind,
      module_name: module_name.into(),
      source: source.into(),
      message: message.into(),
      span: span.into(),
    }
  }

  pub fn syntax(
    module_name: impl Into<Arc<str>>,
    source: impl Into<Arc<str>>,
    message: impl Into<Arc<str>>,
    span: impl Into<Span>,
  ) -> Self {
    Self::new(ErrorKind::Syntax, module_name, source, message, span)
  }

  pub fn emit(
    module_name: impl Into<Arc<str>>,
    source: impl Into<Arc<str>>,
    message: impl Into<Arc<str>>,
    span: impl Into<Span>,
  ) -> Self {
    Self::new(ErrorKind::Emit, module_name, source, message, span)
  }

  pub fn runtime(
    module_name: impl Into<Arc<str>>,
    source: impl Into<Arc<str>>,
    message: impl Into<Arc<str>>,
    span: impl Into<Span>,
  ) -> Self {
    Self::new(ErrorKind::Runtime, module_name, source, message, span)
  }

  pub fn simple(message: impl Into<Arc<str>>) -> Self {
    Self::new(ErrorKind::Simple, "", "", message, Span::empty())
  }

  pub fn kind(&self) -> ErrorKind {
    self.kind
  }

  pub fn module_name(&self) -> &str {
    &self.module_name
  }

  pub fn source(&self) -> &str {
    &self.source
  }

  pub fn message(&self) -> &str {
    &self.message
  }

  pub fn span(&self) -> Span {
    self.span
  }

  pub fn location(&self) -> Location {
    Location::from_source_span(&self.source, &self.span)
  }

  pub fn report(&self) -> String {
    report(
      self.kind,
      &self.module_name,
      &self.source,
      &self.message,
      self.span,
    )
  }
}

pub fn report(
  kind: ErrorKind,
  module_name: &str,
  source: &str,
  message: &str,
  span: Span,
) -> String {
  use core::fmt::Write;

  if matches!(kind, ErrorKind::Simple) {
    return format!("error: {}", message);
  }

  // empty span
  if span.start == span.end {
    return format!("error in `{module_name}`: {message}");
  }

  let mut out = String::new();

  let loc = Location::from_source_span(source, &span);
  let ln = loc.line_num;
  let lw = num_digits(loc.line_num);
  let pos = span.start() - loc.line_start;
  let len = if span.end() > loc.line_end {
    loc.line_end - span.start()
  } else {
    span.end() - span.start()
  };
  let line = &source[loc.line_start..loc.line_end];

  writeln!(&mut out, "error in `{module_name}`: {message}").unwrap();
  writeln!(&mut out, "{ln} |  {line}").unwrap();
  writeln!(&mut out, "{:lw$} |  {:pos$}{:^<len$}", "", "", "^").unwrap();

  out
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
  Simple,
  Syntax,
  Emit,
  Runtime,
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
  line_num: usize,
  column: usize,
  line_start: usize,
  line_end: usize,
}

impl Location {
  fn from_source_span(source: &str, span: &Span) -> Self {
    let line_start = source[..span.start()]
      .rfind('\n')
      .map(|v| v + 1)
      .unwrap_or(0);
    let line_num = 1 + source[..line_start].lines().count();
    let column = span.start() - line_start;
    let line_end = source[span.start()..]
      .find('\n')
      .map(|v| v + span.start())
      .unwrap_or(source.len());

    Self {
      line_num,
      column,
      line_start,
      line_end,
    }
  }

  pub fn line(&self) -> usize {
    self.line_num
  }

  pub fn column(&self) -> usize {
    self.column
  }
}

impl Display for ErrorKind {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      ErrorKind::Simple => Ok(()),
      ErrorKind::Syntax => f.write_str("syntax"),
      ErrorKind::Emit => f.write_str("emit"),
      ErrorKind::Runtime => f.write_str("runtime"),
    }
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let loc = self.location();

    match self.kind {
      ErrorKind::Simple => write!(f, "error: {}", self.message),
      _ => write!(
        f,
        "{} error in {} (line {}, col {}): {}",
        self.kind, self.module_name, loc.line_num, loc.column, self.message
      ),
    }
  }
}

impl Debug for Error {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Error")
      .field("kind", &self.kind)
      .field("module_name", &self.module_name)
      .field("message", &self.message)
      .field("span", &self.span)
      .finish_non_exhaustive()
  }
}

impl std::error::Error for Error {}

fn num_digits(v: usize) -> usize {
  use core::iter::successors;

  successors(Some(v), |&n| (n >= 10).then_some(n / 10)).count()
}
