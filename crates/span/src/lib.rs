//! This module contains the implementation of spans for Mu,
//! and various utilities for working with them.

use std::ops::{Deref, DerefMut, Range};

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

impl From<(usize, usize)> for Span {
  fn from(value: (usize, usize)) -> Self {
    Span {
      start: value.0,
      end: value.1,
    }
  }
}

impl From<Span> for (usize, usize) {
  fn from(value: Span) -> Self {
    (value.start, value.end)
  }
}

impl std::fmt::Display for Span {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}..{}", self.start, self.end)
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

#[cfg(test)]
mod tests {
  use super::*;

  #[allow(clippy::no_effect)]
  #[test]
  fn test_spanned() {
    #[derive(Default)]
    struct Nested {
      v: i32,
    }
    #[derive(Default)]
    struct Test {
      a: i32,
      b: i32,
      c: i32,
      nested: Nested,
    }

    let mut t = Spanned::new(0..10, Test::default());

    t.span.start;
    t.span.end;
    t.a;
    t.b;
    t.c;
    t.nested.v = 10;
  }
}
