use crate::value::object::{Access, Handle, Key, Method};
use crate::value::Value;
use crate::{Error, Result};

pub fn set(receiver: &mut Value, key: &str, value: Value) -> Result<()> {
  if let Some(mut obj) = receiver.clone().to_object_raw() {
    let key = Key::from(key);
    if obj.field_get(&key)?.is_some() || !obj.is_frozen() {
      obj.field_set(key.to_static(), value)?;
      return Ok(());
    }
  };

  Err(Error::new(
    format!("cannot set field `{key}` on value `{receiver}`"),
    0..0,
  ))
}

pub fn get(receiver: &Value, key: &str) -> Result<Value> {
  if let Some(o) = receiver.clone().to_object_raw() {
    let key = Key::from(key);
    if let Some(value) = o.field_get(&key)? {
      if o.should_bind_methods() && is_fn_like(&value) {
        return Ok(Value::object(Handle::alloc(Method::new(
          receiver.clone(),
          value,
        ))));
      }
      return Ok(value);
    }
  }

  Err(Error::new(
    format!("cannot get field `{key}` on value `{receiver}`"),
    0..0,
  ))
}

pub fn get_opt(receiver: &Value, key: &str) -> Result<Value> {
  // early exit if on `none`
  if receiver.is_none() {
    return Ok(Value::none());
  }

  if let Some(o) = receiver.clone().to_object_raw() {
    let key = Key::from(key);
    if let Some(value) = o.field_get(&key)? {
      if o.should_bind_methods() && is_fn_like(&value) {
        return Ok(Value::object(Handle::alloc(Method::new(
          receiver.clone(),
          value,
        ))));
      }
      return Ok(value);
    }
  }

  Ok(Value::none())
}

fn is_fn_like(v: &Value) -> bool {
  v.is_func() || v.is_closure() || v.is_method()
}
