use std::fmt::Display;

use crate::value::handle::Handle;
use crate::value::object::Str;
use crate::value::Value;

pub fn stringify(value: Value) -> DeferString {
  let inner = match value.is_str() {
    true => DeferStringInner::String(value.to_str().unwrap()),
    false => DeferStringInner::Other(value),
  };

  DeferString(inner)
}

pub struct DeferString(DeferStringInner);

enum DeferStringInner {
  String(Handle<Str>),
  Other(Value),
}

impl Display for DeferStringInner {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DeferStringInner::String(s) => write!(f, "{}", s.as_str()),
      DeferStringInner::Other(v) => write!(f, "{v}"),
    }
  }
}

impl Display for DeferString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
