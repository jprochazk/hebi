use std::fmt::{Debug, Display};
use std::ops::Deref;

use beef::lean::Cow;

use super::Object;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct String {
  data: Cow<'static, str>,
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
