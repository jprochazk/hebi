use std::error::Error as StdError;
use std::fmt::{Debug, Display, Write};

use crate::span::Span;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct Error {
  pub span: Span,
  pub message: String,
}

impl Error {
  pub fn new(message: impl ToString, span: impl Into<Span>) -> Self {
    Self {
      span: span.into(),
      message: message.to_string(),
    }
  }

  pub fn report(&self, src: &str, use_color: bool) -> String {
    if self.span.is_empty() {
      return self.message.clone();
    }
    if self.span.start > src.len() || self.span.end > src.len() {
      panic!("invalid span {self}");
    }

    let start = src[..self.span.start].rfind('\n').unwrap_or(0);
    let end = src[self.span.end..]
      .find('\n')
      .map(|v| v + self.span.end)
      .unwrap_or(src.len());

    // print snippet
    let (r, c) = if use_color {
      ("\x1b[0m", "\x1b[4;31m")
    } else {
      ("", "")
    };

    let pre = &src[start..self.span.start].trim_start();
    let content = &src[self.span.start..self.span.end];
    let post = &src[self.span.end..end].trim_end();

    let mut out = String::new();
    let f = &mut out;

    writeln!(f, "{}", self.message).unwrap();
    let mut lines = content.lines().peekable();
    let line = lines.next().unwrap().or("_");
    if lines.peek().is_some() {
      writeln!(f, "| {pre}{c}{line}{r}").unwrap();
      while let Some(line) = lines.next() {
        let line = line.or("_");
        if lines.peek().is_some() {
          writeln!(f, "| {c}{line}{r}").unwrap();
        } else {
          write!(f, "| {c}{line}{r}{post}").unwrap();
        }
      }
    } else {
      writeln!(f, "| {pre}{c}{line}{r}{post}").unwrap();
    }

    out
  }
}

trait EmptyOr {
  fn or<'a>(&'a self, v: &'a Self) -> &'a Self;
}

impl EmptyOr for str {
  fn or<'a>(&'a self, v: &'a Self) -> &'a Self {
    if self.is_empty() {
      v
    } else {
      self
    }
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl StdError for Error {}

#[cfg(test)]
mod tests;
