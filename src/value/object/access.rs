use super::Str;
use crate::ctx::Context;
use crate::value::handle::Handle;
use crate::value::Value;
use crate::{Error, Result};

pub trait Access {
  fn is_frozen(&self) -> bool {
    true
  }

  fn should_bind_methods(&self) -> bool {
    true
  }

  /// Represents the `obj.key` operation.
  fn field_get(&self, ctx: &Context, key: &str) -> Result<Option<Value>> {
    Err(Error::runtime(format!("cannot get field `{key}`")))
  }

  /// Represents the `obj.key = value` operation.
  fn field_set(&mut self, ctx: &Context, key: Handle<Str>, value: Value) -> Result<()> {
    drop(value);
    Err(Error::runtime(format!("cannot set field `{key}`")))
  }

  /// Represents the `obj[key]` operation.
  fn index_get(&self, ctx: &Context, key: Value) -> Result<Option<Value>> {
    Err(Error::runtime(format!("cannot get index `{key}`")))
  }

  /// Represents the `obj[key] = value` operation.
  fn index_set(&mut self, ctx: &Context, key: Value, value: Value) -> Result<()> {
    drop(value);
    Err(Error::runtime(format!("cannot set index `{key}`")))
  }
}

macro_rules! impl_index_via_field {
  (mut) => {
    impl_index_via_field!();

    fn index_set(
      &mut self,
      ctx: &Context,
      key: $crate::value::Value,
      value: $crate::value::Value,
    ) -> $crate::Result<()> {
      match key.clone().to_str() {
        Some(key) => self.field_set(ctx, key, value),
        None => Err($crate::Error::runtime(format!("cannot set index `{key}`"))),
      }
    }
  };
  () => {
    fn index_get(
      &self,
      ctx: &Context,
      key: $crate::value::Value,
    ) -> $crate::Result<Option<$crate::value::Value>> {
      match key.to_str() {
        Some(key) => self.field_get(ctx, key.as_str()),
        None => Ok(None),
      }
    }
  };
}
