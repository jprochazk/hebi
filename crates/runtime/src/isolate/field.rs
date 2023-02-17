use super::error::Error;
use crate::value::object::dict::Key;
use crate::value::object::{Access, Method};
use crate::value::Value;

pub fn set(obj: &mut Value, key: &str, value: Value) -> Result<(), Error> {
  if let Some(obj) = obj.as_object_mut() {
    let key = Key::from(key);
    if obj.field_get(&key)?.is_some() || !obj.is_frozen() {
      obj.field_set(key.to_static(), value)?;
      return Ok(());
    }
  };

  Err(Error::new(format!(
    "cannot set field `{key}` on value `{obj}`"
  )))
}

pub fn get(obj: &Value, key: &str) -> Result<Value, Error> {
  if let Some(o) = obj.as_object() {
    let key = Key::from(key);
    if let Some(value) = o.field_get(&key)? {
      if o.should_bind_methods() && is_fn_like(&value) {
        return Ok(
          Method {
            this: obj.clone(),
            func: value,
          }
          .into(),
        );
      }
      return Ok(value);
    }
  }

  Err(Error::new(format!(
    "cannot get field `{key}` on value `{obj}`"
  )))
}

pub fn get_opt(obj: &Value, key: &str) -> Result<Value, Error> {
  // early exit if on `none`
  if obj.is_none() {
    return Ok(Value::none());
  }

  if let Some(o) = obj.as_object() {
    let key = Key::from(key);
    if let Some(value) = o.field_get(&key)? {
      if o.should_bind_methods() && is_fn_like(&value) {
        return Ok(
          Method {
            this: obj.clone(),
            func: value,
          }
          .into(),
        );
      }
      return Ok(value);
    }
  }

  Ok(Value::none())
}

fn is_fn_like(v: &Value) -> bool {
  v.is_func() || v.is_closure() || v.is_method()
}
