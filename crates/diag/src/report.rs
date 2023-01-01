//! Error reporting
//!
//! The entrypoint to this module is [Report][`crate::Report`]

use std::fmt;
use std::fmt::{Display, Write};
use std::ops::Range;

use beef::lean::Cow;
use owo_colors as colors;
use span::Span;
use thiserror::Error;
use util::leading_whitespace;

use crate::snippet::Snippet;
use crate::source::Source;
use crate::{style, util};

#[derive(Clone)]
pub struct ReportBuilder<'a, Src, Msg, Sp> {
  level: Level,
  source: Src,
  message: Msg,
  span: Sp,
  label: Option<Cow<'a, str>>,
  color: bool,
}

impl<'a, Msg, Sp> ReportBuilder<'a, (), Msg, Sp> {
  pub fn source(self, source: impl Into<Source<'a>>) -> ReportBuilder<'a, Source<'a>, Msg, Sp> {
    let source = source.into();
    ReportBuilder {
      level: self.level,
      source,
      message: self.message,
      span: self.span,
      label: self.label,
      color: self.color,
    }
  }
}

impl<'a, Src, Sp> ReportBuilder<'a, Src, (), Sp> {
  pub fn message(
    self,
    message: impl Into<Cow<'a, str>>,
  ) -> ReportBuilder<'a, Src, Cow<'a, str>, Sp> {
    let message = message.into();
    ReportBuilder {
      level: self.level,
      source: self.source,
      message,
      span: self.span,
      label: self.label,
      color: self.color,
    }
  }
}

impl<'a, Src, Msg> ReportBuilder<'a, Src, Msg, ()> {
  pub fn span(self, span: impl Into<Span>) -> ReportBuilder<'a, Src, Msg, Span> {
    let span = span.into();
    ReportBuilder {
      level: self.level,
      source: self.source,
      message: self.message,
      span,
      label: self.label,
      color: self.color,
    }
  }
}

impl<'a, Src, Msg, Sp> ReportBuilder<'a, Src, Msg, Sp> {
  pub fn label(mut self, label: impl Into<Cow<'a, str>>) -> Self {
    self.label = Some(label.into());
    self
  }

  pub fn color(mut self, enabled: bool) -> Self {
    self.color = enabled;
    self
  }
}

impl<'a> ReportBuilder<'a, Source<'a>, Cow<'a, str>, Span> {
  pub fn build(self) -> Report<'a> {
    Report {
      level: self.level,
      source: self.source,
      message: self.message,
      span: self.span,
      label: self.label,
      color: self.color,
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum Level {
  Info,
  Warning,
  Error,
}

/// Represents a single error that may be emitted to
/// anything which implements [`fmt::Write`].
pub struct Report<'a> {
  pub level: Level,
  pub source: Source<'a>,
  pub message: Cow<'a, str>,
  pub span: Span,
  pub label: Option<Cow<'a, str>>,
  pub color: bool,
}

impl<'a> Report<'a> {
  /// An `Info`-level report.
  pub fn info() -> ReportBuilder<'a, (), (), ()> {
    ReportBuilder {
      level: Level::Info,
      source: (),
      message: (),
      span: (),
      label: None,
      color: true,
    }
  }

  /// A `Warning`-level report.
  pub fn warn() -> ReportBuilder<'a, (), (), ()> {
    ReportBuilder {
      level: Level::Warning,
      source: (),
      message: (),
      span: (),
      label: None,
      color: true,
    }
  }

  /// An `Error`-level report.
  pub fn error() -> ReportBuilder<'a, (), (), ()> {
    ReportBuilder {
      level: Level::Error,
      source: (),
      message: (),
      span: (),
      label: None,
      color: true,
    }
  }

  /// Emit the report to `w`.
  pub fn emit<W: Write>(self, w: &mut W) -> Result<(), EmitError> {
    // examples:

    // single line span
    // error: expected semicolon
    // > foo.mu:11
    // |
    // | let y = 30_
    // |

    // multi-line span
    // error: mismatched type
    // > bar.mu:31
    // |
    // | let x: Foo = Bar {
    // |   a: 0,
    // |   b: 0,
    // | }
    // |
    // + expected "Foo", found "Bar"

    if self.source.str().get(Range::from(self.span)).is_none() {
      return Err(EmitError::OutOfBounds);
    }

    let style = style::Style {
      enabled: self.color,
      span: match self.level {
        Level::Info => colors::style().blue(),
        Level::Warning => colors::style().yellow(),
        Level::Error => colors::style().red(),
      }
      .underline(),
      level: match self.level {
        Level::Info => colors::style().blue(),
        Level::Warning => colors::style().yellow(),
        Level::Error => colors::style().red(),
      },
      symbol: colors::style().blue(),
    };

    let snippet = Snippet::new(self.source.str(), self.span);
    let pipe = style.symbol("|");

    // {level}: {message}
    writeln!(
      w,
      "{}: {}",
      style.level(format!("{}", self.level)),
      self.message
    )?;
    // > {file.name}:{line}
    writeln!(
      w,
      "{} {}:{}",
      style.symbol(">"),
      self.source.name().unwrap_or("code"),
      snippet.line
    )?;

    // empty line to give the snippet some room
    writeln!(w, "{pipe} ")?;
    if snippet.s[Range::from(snippet.span)].trim().is_empty() {
      // "whitespace-only" snippet, may happen in case the span is 1 character wide
      // and lands on a newline.
      let n = snippet.s[Range::from(snippet.span)].len();
      // | {text}{fake_underscore}
      writeln!(
        w,
        "{} {}{}",
        pipe,
        &snippet.s[..snippet.span.start],
        style.span(&format!("{:_<width$}", "_", width = n)),
      )?;
    } else if snippet.count > 1 {
      // multi-line snippet
      // this one is complex to render, but the basic idea is that we want to
      // highlight (using color and an underline) the part of the multi-line snippet
      // which is spanned.

      // begin by finding the first and last lines,
      // since we know we have at least 2 lines.
      let first_lf = snippet.s[snippet.span.start..]
        .find('\n')
        .map(|i| i + snippet.span.start)
        .unwrap_or(snippet.s.len());
      let last_lf = snippet.s[snippet.span.start..]
        .rfind('\n')
        .map(|i| i + snippet.span.start)
        .unwrap_or(snippet.s.len());

      // write the first line, this one contains both an "unspanned" fragment,
      // and a "spanned" fragment.
      // | {unspanned}{spanned}
      writeln!(
        w,
        "{} {}{}",
        pipe,
        &snippet.s[..snippet.span.start].trim_start(), // unspanned
        style.span(
          snippet.s[snippet.span.start..first_lf] // spanned
            .trim_end()
        )
      )?;

      // write the lines in between the first and the last.
      // it is separated like this because this part of the snippet is entirely
      // contained within the span, so all of it will be colored and underlined.
      match snippet.count {
        // write all the lines in the middle if we have 3..=5 total lines.
        3..=5 => {
          for line in snippet.s[first_lf..last_lf].split('\n').skip(1) {
            writeln!(w, "{} {}", pipe, style.span(line))?;
          }
        }
        // if we have more than 6 lines, this is considered a very large snippet, and it probably
        // isn't useful to show all of it, so we only print the first and last lines of this
        // "middle" region + an extra line with some dots that represents all the truncated content.
        6.. => {
          let mut iter = snippet.s[first_lf..last_lf].split('\n');
          iter.next(); // skip line 1, because it's empty.
                       // this is safer than indexing `snippet.s` with `first_lf+1..last_lf`
          let first = iter.next().unwrap();

          let mut iter = iter.rev();
          let last = iter.next().unwrap();
          let ws = leading_whitespace(first);

          // |   b: 0,
          // |   ...
          // |   h: 0,
          writeln!(w, "{} {}", pipe, style.span(first))?;
          writeln!(w, "{} {}{}", pipe, style.span(ws), style.span("..."))?;
          writeln!(w, "{} {}", pipe, style.span(last))?;
        }
        _ => {}
      }

      // write the last line the same way the first one is written, but in reverse.
      // | {spanned}{unspanned}
      writeln!(
        w,
        "{} {}{}",
        pipe,
        style.span(
          snippet.s[last_lf.min(snippet.span.end)..snippet.span.end] // spanned
            .trim_start()
        ),
        &snippet.s[snippet.span.end..].trim_end(), // unspanned
      )?;
    } else {
      // single-line snippet
      // | {text}{spanned_text}{text}
      writeln!(
        w,
        "{} {}{}{}",
        pipe,
        &snippet.s[..snippet.span.start],
        style.span(&snippet.s[Range::from(snippet.span)]),
        &snippet.s[snippet.span.end..].trim_end()
      )?;
    }
    // empty line at the end for symmetry
    writeln!(w, "{pipe} ")?;
    if let Some(label) = self.label {
      // + {label}
      writeln!(w, "{} {}", style.symbol("+"), label)?;
    }

    Ok(())
  }

  /// Emit the report to a string.
  pub fn emit_to_string(self) -> Result<String, EmitError> {
    let mut buf = String::new();
    self.emit(&mut buf)?;
    Ok(buf)
  }
}

#[derive(Debug, Error)]
pub enum EmitError {
  #[error("failed to format")]
  Fmt(#[from] fmt::Error),
  #[error("span does not fit within source string")]
  OutOfBounds,
}

impl Display for Level {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let level = match self {
      Level::Info => "info",
      Level::Warning => "warning",
      Level::Error => "error",
    };
    write!(f, "{level}")
  }
}

#[cfg(test)]
mod tests;
