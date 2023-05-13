use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::ops::Deref;

use beef::lean::Cow;

use super::{Object, Ptr};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct String {
  data: Cow<'static, str>,
}

impl String {
  pub fn new(data: Cow<'static, str>) -> Self {
    Self { data }
  }

  pub fn as_str(&self) -> &str {
    self.data.as_ref()
  }
}

impl Object for String {
  fn type_name(&self) -> &'static str {
    "String"
  }
}

impl Display for String {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.data, f)
  }
}

impl Debug for String {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.data, f)
  }
}

impl Deref for String {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    self.data.as_ref()
  }
}

impl std::borrow::Borrow<str> for String {
  fn borrow(&self) -> &str {
    self.data.borrow()
  }
}

impl AsRef<str> for String {
  fn as_ref(&self) -> &str {
    self.data.as_ref()
  }
}

impl indexmap::Equivalent<str> for Ptr<String> {
  fn equivalent(&self, key: &str) -> bool {
    self.as_str() == key
  }
}

impl Borrow<str> for Ptr<String> {
  fn borrow(&self) -> &str {
    self
  }
}

impl<'a> PartialEq<&'a str> for Ptr<String> {
  fn eq(&self, other: &&'a str) -> bool {
    self.as_str() == *other
  }
}