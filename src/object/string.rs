use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::ops::Deref;

use beef::lean::Cow;

use super::{Object, Ptr};
use crate::value::Value;
use crate::{Result, Scope};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Str {
  data: Cow<'static, str>,
}

impl Str {
  #[allow(dead_code)] // symmetry with `owned`
  pub fn borrowed(data: &'static str) -> Self {
    Self {
      data: Cow::borrowed(data),
    }
  }

  pub fn owned(data: impl ToString) -> Self {
    Self {
      data: Cow::owned(data.to_string()),
    }
  }

  pub fn as_str(&self) -> &str {
    self.data.as_ref()
  }

  pub fn concat(&self, other: &str) -> Self {
    let mut out = String::with_capacity(self.len() + other.len());
    out.push_str(self.as_str());
    out.push_str(other);
    Self::owned(out)
  }
}

impl Object for Str {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "String"
  }

  default_instance_of!();

  fn add(scope: Scope<'_>, this: Ptr<Self>, other: Ptr<Self>) -> Result<Value> {
    Ok(Value::object(scope.alloc(this.concat(other.as_str()))))
  }
}

declare_object_type!(Str);

impl Display for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.data, f)
  }
}

impl Debug for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.data, f)
  }
}

impl Deref for Str {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    self.data.as_ref()
  }
}

impl std::borrow::Borrow<str> for Str {
  fn borrow(&self) -> &str {
    self.data.borrow()
  }
}

impl AsRef<str> for Str {
  fn as_ref(&self) -> &str {
    self.data.as_ref()
  }
}

impl indexmap::Equivalent<str> for Ptr<Str> {
  fn equivalent(&self, key: &str) -> bool {
    self.as_str() == key
  }
}

impl Borrow<str> for Ptr<Str> {
  fn borrow(&self) -> &str {
    self
  }
}

impl<'a> PartialEq<&'a str> for Ptr<Str> {
  fn eq(&self, other: &&'a str) -> bool {
    self.as_str() == *other
  }
}
