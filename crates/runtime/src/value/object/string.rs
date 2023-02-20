use std::fmt::Display;

use beef::lean::Cow;
use derive::delegate_to_handle;

use super::{Access, Key};
use crate::Value;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Str(String);

#[delegate_to_handle]
impl Str {
  pub(crate) fn as_str(&self) -> &str {
    self.0.as_str()
  }
}

impl<'a> From<Cow<'a, str>> for Str {
  fn from(value: Cow<'a, str>) -> Self {
    Self(value.to_string())
  }
}

impl From<String> for Str {
  fn from(value: String) -> Self {
    Self(value)
  }
}

impl<'a> From<&'a str> for Str {
  fn from(value: &'a str) -> Self {
    Self(value.to_string())
  }
}

impl Access for Str {
  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    Ok(match key.as_str() {
      Some("len") => Some(Value::int(self.0.len() as i32)),
      _ => None,
    })
  }
}

impl Display for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "\"{}\"", self.0)
  }
}
