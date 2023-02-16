use std::fmt::{Debug, Display, Write};

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Str(String);

impl Str {
  pub fn new() -> Self {
    Self(String::new())
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self(String::with_capacity(capacity))
  }

  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

impl From<String> for Str {
  fn from(value: String) -> Self {
    Self(value)
  }
}

impl<'a> From<&'a str> for Str {
  fn from(value: &'a str) -> Self {
    Self(value.into())
  }
}

impl Display for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.0, f)
  }
}

impl Debug for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.0, f)
  }
}

impl Write for Str {
  fn write_str(&mut self, s: &str) -> std::fmt::Result {
    self.0.write_str(s)
  }
}
