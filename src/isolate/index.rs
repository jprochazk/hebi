use crate::ctx::Context;
use crate::value::object::Access;
use crate::value::Value;
use crate::{Error, Result};

pub fn set(ctx: Context, obj: Value, key: Value, value: Value) -> Result<()> {
  if let Some(mut obj) = obj.clone().to_object_raw() {
    if obj.index_get(key.clone())?.is_some() || !obj.is_frozen() {
      obj.index_set(key, value)?;
      return Ok(());
    }
  };

  Err(Error::runtime(format!(
    "cannot set field `{key}` on value `{obj}`"
  )))
}

pub fn get(obj: Value, key: Value) -> Result<Value> {
  if let Some(o) = obj.clone().to_object_raw() {
    if let Some(value) = o.index_get(key.clone())? {
      return Ok(value);
    }
  }

  Err(Error::runtime(format!(
    "cannot get field `{key}` on value `{obj}`"
  )))
}

pub fn get_opt(obj: Value, key: Value) -> Result<Value> {
  // early exit if on `none`
  if obj.is_none() {
    return Ok(Value::none());
  }

  if let Some(o) = obj.clone().to_object_raw() {
    if let Some(value) = o.index_get(key)? {
      return Ok(value);
    }
  }

  Ok(Value::none())
}
