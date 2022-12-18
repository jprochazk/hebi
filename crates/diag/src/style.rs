use std::fmt;
use std::fmt::Display;

use owo_colors as colors;

pub struct Style {
  pub enabled: bool,
  pub span: colors::Style,
  pub level: colors::Style,
  pub symbol: colors::Style,
}

impl Style {
  pub fn span<'a, T: Display + 'a>(&'a self, inner: T) -> Styled<'a, T> {
    Styled {
      inner,
      style: self.enabled.then_some(&self.span),
    }
  }

  pub fn level<'a, T: Display + 'a>(&'a self, inner: T) -> Styled<'a, T> {
    Styled {
      inner,
      style: self.enabled.then_some(&self.level),
    }
  }

  pub fn symbol<'a, T: Display + 'a>(&'a self, inner: T) -> Styled<'a, T> {
    Styled {
      inner,
      style: self.enabled.then_some(&self.symbol),
    }
  }
}

pub struct Styled<'a, T: Display + 'a> {
  inner: T,
  style: Option<&'a colors::Style>,
}

impl<'a, T: Display> Display for Styled<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    use colors::OwoColorize;

    if let Some(style) = self.style {
      write!(f, "{}", self.inner.style(*style))
    } else {
      write!(f, "{}", self.inner)
    }
  }
}
