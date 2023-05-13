//! This module contains the implementation of spans for Hebi,
//! and various utilities for working with them.

use std::error::Error as StdError;
use std::fmt::{Debug, Display, Write};
use std::ops::{Deref, DerefMut, Index, Range};

/// Represents a span of bytes in some source string.
///
/// This type is just like [`std::ops::Range<usize>`],
/// but unlike the standard Range, it is marked [`std::marker::Copy`].
///
/// It is used for highlighting code in emitted diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
  pub start: usize,
  pub end: usize,
}

impl Span {
  /// Create a new span which starts at `self.start` and ends at `other.end`.
  pub fn join(&self, other: Span) -> Span {
    Span {
      start: self.start,
      end: other.end,
    }
  }

  pub fn range(&self) -> Range<usize> {
    Range {
      start: self.start,
      end: self.end,
    }
  }

  pub fn is_empty(&self) -> bool {
    self.start == self.end
  }
}

impl From<Range<usize>> for Span {
  fn from(value: Range<usize>) -> Self {
    Self {
      start: value.start,
      end: value.end,
    }
  }
}

impl From<Span> for Range<usize> {
  fn from(value: Span) -> Self {
    Range {
      start: value.start,
      end: value.end,
    }
  }
}

impl std::fmt::Display for Span {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}..{}", self.start, self.end)
  }
}

impl Index<Span> for [u8] {
  type Output = [u8];

  fn index(&self, index: Span) -> &Self::Output {
    &self[index.range()]
  }
}

impl Index<Span> for str {
  type Output = str;

  fn index(&self, index: Span) -> &Self::Output {
    &self[index.range()]
  }
}

/// Represents a value, and its span in the source string from which it was
/// parsed. This allows tracing syntax nodes back to their location in the
/// source string.
#[derive(Clone, Copy, Default)]
pub struct Spanned<T> {
  pub span: Span,
  value: T,
}

impl<T> Spanned<T> {
  pub fn new(span: impl Into<Span>, value: T) -> Spanned<T> {
    Spanned {
      span: span.into(),
      value,
    }
  }

  pub fn into_inner(self) -> T {
    self.value
  }

  /// Maps `Spanned<T>` to `Spanned<U>` by calling `f` with `T`,
  /// and preserving the span.
  ///
  /// Useful when wrapping values in nodes, such as in the case of `ExprKind`
  #[inline]
  pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
    Spanned {
      span: self.span,
      value: f(self.value),
    }
  }

  /// Maps `Spanned<T>` to `Spanned<U>` by calling `f` with `Spanned<T>`,
  /// and preserving the span.
  ///
  /// Useful when constructing nodes that require inner nodes to be spanned,
  /// such as in the case of `stmt_expr`.
  #[inline]
  pub fn then<U>(self, f: impl FnOnce(Spanned<T>) -> U) -> Spanned<U> {
    Spanned {
      span: self.span,
      value: f(self),
    }
  }
}

impl<T> Deref for Spanned<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.value
  }
}

impl<T> DerefMut for Spanned<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.value
  }
}

impl<T: std::hash::Hash> std::hash::Hash for Spanned<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    // self.span.hash(state);
    self.value.hash(state);
  }
}

impl<T: std::cmp::PartialEq> std::cmp::PartialEq for Spanned<T> {
  fn eq(&self, other: &Self) -> bool {
    /* self.span == other.span && */
    self.value == other.value
  }
}

impl<T: std::cmp::Eq> std::cmp::Eq for Spanned<T> {}

impl<T: std::cmp::PartialOrd> std::cmp::PartialOrd for Spanned<T> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    /* match self.span.partial_cmp(&other.span) {
        Some(core::cmp::Ordering::Equal) => {}
        ord => return ord,
    } */
    self.value.partial_cmp(&other.value)
  }
}

impl<T: std::cmp::Ord> std::cmp::Ord for Spanned<T> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.value.cmp(&other.value)
  }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Spanned<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.value.fmt(f)
  }
}

impl<T: std::fmt::Display> std::fmt::Display for Spanned<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.value.fmt(f)
  }
}

#[derive(Clone, Debug)]
pub struct SpannedError {
  pub span: Span,
  pub message: String,
}

pub trait MaybeSpan {
  fn into_span(self) -> Span;
}

impl MaybeSpan for Span {
  fn into_span(self) -> Span {
    self
  }
}

impl MaybeSpan for Range<usize> {
  fn into_span(self) -> Span {
    self.into()
  }
}

impl MaybeSpan for Option<Range<usize>> {
  fn into_span(self) -> Span {
    match self {
      Some(v) => v.into(),
      None => (0..0).into(),
    }
  }
}

impl SpannedError {
  pub fn new(message: impl ToString, span: impl MaybeSpan) -> Self {
    Self {
      span: span.into_span(),
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

impl Display for SpannedError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl StdError for SpannedError {}

#[cfg(all(test, not(feature = "__miri")))]
mod tests;
