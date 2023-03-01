use std::fmt::Display;

use super::Str;
use crate::value::handle::Handle;
use crate::value::Value;

pub enum OwnedIndex {
  Str(Handle<Str>),
  Int(i32),
}

impl Display for OwnedIndex {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      OwnedIndex::Str(key) => write!(f, "{key}"),
      OwnedIndex::Int(key) => write!(f, "{key}"),
    }
  }
}

pub enum IndexRef<'a> {
  Str(&'a str),
  Int(i32),
}

impl<'a> Display for IndexRef<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      IndexRef::Str(key) => write!(f, "{key}"),
      IndexRef::Int(key) => write!(f, "{key}"),
    }
  }
}

pub trait Access {
  fn is_frozen(&self) -> bool {
    true
  }

  fn should_bind_methods(&self) -> bool {
    true
  }

  /// Represents the `obj.key` operation.
  fn field_get(&self, key: &str) -> crate::Result<Option<Value>> {
    Err(crate::RuntimeError::script(
      format!("cannot get field `{key}`"),
      0..0,
    ))
  }

  /// Represents the `obj.key = value` operation.
  fn field_set(&mut self, key: Handle<Str>, value: Value) -> crate::Result<()> {
    drop(value);
    Err(crate::RuntimeError::script(
      format!("cannot set field `{key}`"),
      0..0,
    ))
  }

  /// Represents the `obj[key]` operation.
  fn index_get(&self, key: Value) -> crate::Result<Option<Value>> {
    Err(crate::RuntimeError::script(
      format!("cannot get index `{key}`"),
      0..0,
    ))
  }

  /// Represents the `obj[key] = value` operation.
  fn index_set(&mut self, key: Value, value: Value) -> crate::Result<()> {
    drop(value);
    Err(crate::RuntimeError::script(
      format!("cannot set index `{key}`"),
      0..0,
    ))
  }
}

macro_rules! impl_index_via_field {
  (mut) => {
    impl_index_via_field!();

    fn index_set(
      &mut self,
      key: $crate::value::Value,
      value: $crate::value::Value,
    ) -> $crate::Result<()> {
      match key.clone().to_str() {
        Some(key) => self.field_set(key, value),
        None => Err(crate::RuntimeError::script(
          format!("cannot set index `{key}`"),
          0..0,
        )),
      }
    }
  };
  () => {
    fn index_get(&self, key: $crate::value::Value) -> $crate::Result<Option<$crate::value::Value>> {
      match key.to_str() {
        Some(key) => self.field_get(key.as_str()),
        None => Ok(None),
      }
    }
  };
}
