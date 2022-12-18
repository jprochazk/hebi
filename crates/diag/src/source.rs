use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct Source<'a> {
  name: Option<Cow<'a, str>>,
  str: Cow<'a, str>,
}

impl<'a> Source<'a> {
  pub fn string(str: impl Into<Cow<'a, str>>) -> Self {
    Source {
      name: None,
      str: str.into(),
    }
  }

  pub fn file(name: impl Into<Cow<'a, str>>, str: impl Into<Cow<'a, str>>) -> Self {
    Source {
      name: Some(name.into()),
      str: str.into(),
    }
  }

  pub fn name(&self) -> Option<&str> {
    self.name.as_deref()
  }

  pub fn str(&self) -> &str {
    self.str.as_ref()
  }
}

impl<'a> From<Cow<'a, str>> for Source<'a> {
  fn from(value: Cow<'a, str>) -> Self {
    Source::string(value)
  }
}

impl<'a> From<String> for Source<'a> {
  fn from(value: String) -> Self {
    Source::string(value)
  }
}

impl<'a> From<&'a str> for Source<'a> {
  fn from(value: &'a str) -> Self {
    Source::string(value)
  }
}
