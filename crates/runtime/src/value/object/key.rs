use std::hash::Hash;

use beef::lean::Cow;
use indexmap::Equivalent;

use super::{Handle, Str};
use crate::Value;

#[derive(Clone)]
pub enum Key<'a> {
  Int(i32),
  Str(Handle<Str>),
  Ref(&'a str),
}

pub type StaticKey = Key<'static>;

impl<'a> Key<'a> {
  pub fn to_int(&self) -> Option<i32> {
    match self {
      Key::Int(v) => Some(*v),
      Key::Str(_) => None,
      Key::Ref(_) => None,
    }
  }

  pub fn to_str(&self) -> Option<Handle<Str>> {
    match &self {
      Key::Str(v) => Some(v.clone()),
      Key::Ref(v) => Some(Handle::alloc(Str::from(*v))),
      Key::Int(_) => None,
    }
  }

  pub(crate) fn as_str(&self) -> Option<&str> {
    match &self {
      Key::Str(v) => Some(v.as_str()),
      Key::Ref(v) => Some(*v),
      Key::Int(_) => None,
    }
  }

  pub(crate) fn write_to_string(&self, s: &mut String) {
    use std::fmt::Write;
    match &self {
      Key::Int(v) => write!(s, "{v}").unwrap(),
      Key::Str(v) => write!(s, "{v}").unwrap(),
      Key::Ref(v) => write!(s, "{v}").unwrap(),
    }
  }

  pub fn to_static(self) -> Key<'static> {
    match self {
      Key::Int(v) => Key::Int(v),
      Key::Str(v) => Key::Str(v),
      Key::Ref(v) => Key::Str(Handle::alloc(Str::from(v))),
    }
  }
}

impl From<i32> for Key<'static> {
  fn from(value: i32) -> Self {
    Key::Int(value)
  }
}

impl<'a> From<&'a str> for Key<'a> {
  fn from(value: &'a str) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key::Ref(value)
  }
}

impl<'a> From<Cow<'a, str>> for Key<'a> {
  fn from(value: Cow<'a, str>) -> Self {
    if value.is_borrowed() {
      Key::Ref(value.unwrap_borrowed())
    } else {
      Key::Str(Handle::alloc(Str::from(value.to_string())))
    }
  }
}

impl<'a> From<String> for Key<'a> {
  fn from(value: String) -> Self {
    Self::Str(Handle::alloc(Str::from(value)))
  }
}

impl<'a> From<Str> for Key<'a> {
  fn from(value: Str) -> Self {
    Self::Str(Handle::alloc(value))
  }
}

impl<'a> From<Handle<Str>> for Key<'a> {
  fn from(value: Handle<Str>) -> Self {
    Self::Str(value)
  }
}

impl<'a> TryFrom<Value> for Key<'a> {
  type Error = InvalidKeyType;

  fn try_from(value: Value) -> Result<Self, Self::Error> {
    if let Some(v) = value.clone().to_int() {
      return Ok(Key::Int(v));
    }
    if let Some(v) = value.to_str() {
      return Ok(Key::Str(v));
    }
    Err(InvalidKeyType)
  }
}

impl<'a> Equivalent<Key<'a>> for str {
  fn equivalent(&self, key: &Key) -> bool {
    match key {
      Key::Int(_) => false,
      Key::Str(v) => v.as_str() == self,
      Key::Ref(v) => *v == self,
    }
  }
}

impl<'a> Equivalent<Key<'a>> for i32 {
  fn equivalent(&self, key: &Key<'a>) -> bool {
    match key {
      Key::Int(v) => self == v,
      Key::Str(_) => false,
      Key::Ref(_) => false,
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct InvalidKeyType;

impl std::fmt::Display for InvalidKeyType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "invalid key type")
  }
}

impl std::error::Error for InvalidKeyType {}

impl<'a> PartialEq for Key<'a> {
  fn eq(&self, other: &Self) -> bool {
    match (&self, &other) {
      (Key::Int(a), Key::Int(b)) => a == b,
      (Key::Str(a), Key::Str(b)) => a.as_str() == b.as_str(),
      (Key::Ref(a), Key::Ref(b)) => a == b,
      (Key::Str(a), Key::Ref(b)) => a.as_str() == *b,
      (Key::Ref(a), Key::Str(b)) => *a == b.as_str(),
      (Key::Int(_), Key::Str(_)) => false,
      (Key::Int(_), Key::Ref(_)) => false,
      (Key::Str(_), Key::Int(_)) => false,
      (Key::Ref(_), Key::Int(_)) => false,
    }
  }
}

impl<'a> Eq for Key<'a> {}

impl<'a> PartialOrd for Key<'a> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    match (&self, &other) {
      (Key::Int(a), Key::Int(b)) => a.partial_cmp(b),
      (Key::Int(_), Key::Str(_)) => Some(std::cmp::Ordering::Less),
      (Key::Str(_), Key::Int(_)) => Some(std::cmp::Ordering::Greater),
      (Key::Str(a), Key::Str(b)) => a.as_str().partial_cmp(b.as_str()),
      (Key::Ref(a), Key::Str(b)) => a.partial_cmp(&b.as_str()),
      (Key::Ref(a), Key::Ref(b)) => a.partial_cmp(b),
      (Key::Str(a), Key::Ref(b)) => a.as_str().partial_cmp(*b),
      (Key::Int(_), Key::Ref(_)) => Some(std::cmp::Ordering::Less),
      (Key::Ref(_), Key::Int(_)) => Some(std::cmp::Ordering::Greater),
    }
  }
}

impl<'a> Ord for Key<'a> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    unsafe { self.partial_cmp(other).unwrap_unchecked() }
  }
}

impl<'a> Hash for Key<'a> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match &self {
      Key::Int(v) => v.hash(state),
      Key::Str(v) => v.as_str().hash(state),
      Key::Ref(v) => (*v).hash(state),
    }
  }
}

impl<'a> std::fmt::Display for Key<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self {
      Key::Int(v) => write!(f, "{v}"),
      Key::Str(v) => write!(f, "\"{}\"", v.as_str()),
      Key::Ref(v) => write!(f, "\"{}\"", *v),
    }
  }
}
