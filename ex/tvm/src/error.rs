use crate::lex::Span;
use crate::lex::TokenKind;
use crate::util::num_digits;
use core::fmt::{self, Debug, Display, Write};

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

#[derive(Clone)]
pub struct Error {
  kind: ErrorKind,
  span: Span,
}

macro_rules! err {
  (@$span:expr, $kind:ident,) => {err!(@$span, $kind)};
  (@$span:expr, $kind:ident) => {
    $crate::error::Error::new(
      $crate::error::ErrorKind::$kind,
      ($span).into(),
    )
  };
  (@$span:expr, $kind:ident, $($f:expr),+) => {
    $crate::error::Error::new(
      $crate::error::ErrorKind::$kind($($f),+),
      ($span).into(),
    )
  };
}

impl Error {
  #[inline]
  pub fn new(kind: ErrorKind, span: Span) -> Self {
    Self { kind, span }
  }

  #[inline]
  pub fn span(&self) -> Span {
    self.span
  }

  #[inline]
  pub fn location(&self, src: &str) -> Location {
    Location::from_source_span(src, &self.span)
  }

  #[inline]
  pub fn with_src<'a, 'src>(&'a self, src: &'src str) -> ErrorWithSrc<'a, 'src> {
    ErrorWithSrc { error: self, src }
  }
}

pub struct ErrorWithSrc<'a, 'src> {
  error: &'a Error,
  src: &'src str,
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

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub enum ErrorKind {
  UnexpectedToken,
  ExpectedToken(TokenKind),
  InvalidFloat,
  InvalidInt,
  InvalidEscape,
}

impl Display for ErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorKind::UnexpectedToken => f.write_str("unexpected token"),
      ErrorKind::ExpectedToken(kind) => write!(f, "expected token `{kind}`"),
      ErrorKind::InvalidFloat => f.write_str("invalid float literal"),
      ErrorKind::InvalidInt => f.write_str("invalid integer literal"),
      ErrorKind::InvalidEscape => f.write_str("invalid string escape"),
    }
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "error: {}", self.kind)
  }
}

impl Debug for Error {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Error")
      .field("kind", &self.kind)
      .field("span", &self.span)
      .finish_non_exhaustive()
  }
}

impl Display for ErrorWithSrc<'_, '_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    report(f, &self.error.kind, self.src, self.error.span)
  }
}

pub fn report(out: &mut impl Write, kind: &ErrorKind, src: &str, span: Span) -> fmt::Result {
  if span.is_empty() {
    return write!(out, "error: {kind}");
  }

  let loc = Location::from_source_span(src, &span);
  let ln = loc.line_num;
  let lw = num_digits(loc.line_num);
  let pos = span.start() - loc.line_start;
  let len = if span.end() > loc.line_end {
    loc.line_end - span.start()
  } else {
    span.end() - span.start()
  };
  let line = &src[loc.line_start..loc.line_end];

  writeln!(out, "error: {kind}")?;
  writeln!(out, "{ln} |  {line}")?;
  writeln!(out, "{:lw$} |  {:pos$}{:^<len$}", "", "", "^")?;

  Ok(())
}
