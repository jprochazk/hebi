use crate::value::object::dict::{Key, StaticKey};
use crate::value::object::Access;
use crate::value::Value;
use crate::{Error, Result};

pub fn set(obj: &mut Value, key: StaticKey, value: Value) -> Result<()> {
  if let Some(obj) = obj.as_object_mut() {
    if obj.index_get(&key)?.is_some() || !obj.is_frozen() {
      obj.index_set(key.to_static(), value)?;
      return Ok(());
    }
  };

  Err(Error::new(
    format!("cannot set field `{key}` on value `{obj}`"),
    0..0,
  ))
}

pub fn get(obj: &Value, key: &Key) -> Result<Value> {
  if let Some(o) = obj.as_object() {
    if let Some(value) = o.index_get(key)? {
      return Ok(value);
    }
  }

  Err(Error::new(
    format!("cannot get field `{key}` on value `{obj}`"),
    0..0,
  ))
}

pub fn get_opt(obj: &Value, key: &Key) -> Result<Value> {
  // early exit if on `none`
  if obj.is_none() {
    return Ok(Value::none());
  }

  if let Some(o) = obj.as_object() {
    if let Some(value) = o.index_get(key)? {
      return Ok(value);
    }
  }

  Ok(Value::none())
}
