use crate::ctx::Context;
use crate::value::object::{Access, Key, Method};
use crate::value::Value;
use crate::{Result, RuntimeError};

pub fn set(ctx: Context, receiver: &mut Value, key: &str, value: Value) -> Result<()> {
  if let Some(mut obj) = receiver.clone().to_object_raw() {
    let key = Key::Ref(key);
    if obj.field_get(&key)?.is_some() || !obj.is_frozen() {
      obj.field_set(key.to_static(ctx), value)?;
      return Ok(());
    }
  };

  Err(RuntimeError::new(
    format!("cannot set field `{key}` on value `{receiver}`"),
    0..0,
  ))
}

pub fn get(ctx: Context, receiver: &Value, key: &str) -> Result<Value> {
  if let Some(o) = receiver.clone().to_object_raw() {
    let key = Key::Ref(key);
    if let Some(value) = o.field_get(&key)? {
      if o.should_bind_methods() && is_fn_like(&value) {
        return Ok(Value::object(
          ctx.alloc(Method::new(receiver.clone(), value)),
        ));
      }
      return Ok(value);
    }
  }

  Err(RuntimeError::new(
    format!("cannot get field `{key}` on value `{receiver}`"),
    0..0,
  ))
}

pub fn get_opt(ctx: Context, receiver: &Value, key: &str) -> Result<Value> {
  // early exit if on `none`
  if receiver.is_none() {
    return Ok(Value::none());
  }

  if let Some(o) = receiver.clone().to_object_raw() {
    let key = Key::Ref(key);
    if let Some(value) = o.field_get(&key)? {
      if o.should_bind_methods() && is_fn_like(&value) {
        return Ok(Value::object(
          ctx.alloc(Method::new(receiver.clone(), value)),
        ));
      }
      return Ok(value);
    }
  }

  Ok(Value::none())
}

fn is_fn_like(v: &Value) -> bool {
  v.is_func() || v.is_closure() || v.is_method()
}
