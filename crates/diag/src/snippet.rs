use std::borrow::Cow;

use span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Snippet<'a> {
  /// Snippet string
  pub s: Cow<'a, str>,
  /// Line number of the first line in snippet
  pub line: usize,
  /// Number of lines in this snippet
  pub count: usize,
  /// The span inside `s` which should be highlighted
  pub span: Span,
}

impl<'a> Snippet<'a> {
  pub fn new(src: &'a str, span: impl Into<Span>) -> Self {
    let span: Span = span.into();
    // the span may be multiple lines, we want to find the "full" snippet which
    // contains all the lines that the span covers.
    // for example (span is `_`):
    //   a
    //   _b
    //   cd
    //   ef_g
    //   hi
    // will yield these lines:
    //   b
    //   cd
    //   efg

    let start_line = src[..span.start].rfind('\n').unwrap_or(0);
    let end_line = src[span.end..]
      .find('\n')
      .unwrap_or_else(|| src[span.end..].len())
      + span.end;
    let original_trailing_whitespace_count = src[..span.end]
      .chars()
      .rev()
      .take_while(|c| c.is_ascii_whitespace() || *c == '\n')
      .count();

    let raw_snippet = &src[start_line..end_line];
    let s = raw_snippet.trim_start_matches('\n');
    let preceding_chars = raw_snippet.len() - s.len();
    let s = s.trim_end_matches(|c: char| c == '\n' || c.is_ascii_whitespace());

    let line = src[..span.start].split('\n').count();
    let count = s.split('\n').count();

    let mut span = Span {
      start: span
        .start
        .saturating_sub(start_line)
        .saturating_sub(preceding_chars),
      end: span
        .end
        .saturating_sub(start_line)
        .saturating_sub(preceding_chars)
        .saturating_sub(original_trailing_whitespace_count),
    };
    span.start = span.start.min(span.end);

    Self {
      s: s.into(),
      line,
      count,
      span,
    }
  }

  #[cfg(test)]
  pub fn highlight(&self) -> &str {
    &self.s[self.span.start..self.span.end]
  }
}
