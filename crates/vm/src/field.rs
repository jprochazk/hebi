use value::object::dict::Key;
use value::object::Method;
use value::Value;

use crate::Error;

pub fn set(obj: &mut Value, key: Key, value: Value) -> Result<(), Error> {
  if let Some(mut class) = obj.as_class_mut() {
    if class.has(&key) {
      class.set(&key, value);
      return Ok(());
    }

    if !class.is_frozen() {
      class.insert(key, value);
      return Ok(());
    }

    return Err(Error::new("cannot add field to frozen class"));
  }

  if let Some(mut dict) = obj.as_dict_mut() {
    dict.insert(key, value);
    return Ok(());
  }

  Err(Error::new(format!(
    "cannot set field `{key}` on value `{obj}`"
  )))
}

pub fn get(obj: &Value, key: &Key) -> Result<Value, Error> {
  if let Some(class) = obj.as_class() {
    let Some(value) = class.get(key).cloned() else {
      return Err(Error::new(format!(
        "cannot get field `{key}` on value `{obj}`"
      )));
    };
    if is_fn_like(&value) {
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

  if let Some(dict) = obj.as_dict() {
    let Some(value) = dict.get(key).cloned() else {
      return Err(Error::new(format!(
        "cannot get field `{key}` on value `{obj}`"
      )));
    };
    return Ok(value);
  }

  if let Some(def) = obj.as_class_def() {
    let Some(value) = def.method(key).cloned() else {
      return Err(Error::new(format!(
        "cannot get field `{key}` on value `{obj}`"
      )));
    };
    return Ok(value);
  }

  if let Some(proxy) = obj.as_proxy() {
    let Some(value) = proxy.parent().borrow().method(key).cloned() else {
      return Err(Error::new(format!(
        "cannot get field `{key}` on value `{obj}`"
      )));
    };
    assert!(is_fn_like(&value));
    let value = Method {
      this: obj.clone(),
      func: value,
    };
    return Ok(value.into());
  }

  Err(Error::new(format!(
    "cannot get field `{key}` on value `{obj}`"
  )))
}

pub fn get_opt(obj: &Value, key: &Key) -> Result<Value, Error> {
  // early exit if on `none`
  if obj.is_none() {
    return Ok(Value::none());
  }

  if let Some(class) = obj.as_class() {
    let Some(value) = class.get(key).cloned() else {
      return Ok(Value::none());
    };
    if is_fn_like(&value) {
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

  if let Some(dict) = obj.as_dict() {
    let Some(value) = dict.get(key).cloned() else {
      return Ok(Value::none());
    };
    return Ok(value);
  }

  if let Some(def) = obj.as_class_def() {
    let Some(value) = def.method(key).cloned() else {
      return Ok(Value::none());
    };
    return Ok(value);
  }

  if let Some(proxy) = obj.as_proxy() {
    let Some(value) = proxy.parent().borrow().method(key).cloned() else {
      return Ok(Value::none());
    };
    assert!(is_fn_like(&value));
    let value = Method {
      this: obj.clone(),
      func: value,
    };
    return Ok(value.into());
  }

  Err(Error::new(format!(
    "cannot get field `{key}` on value `{obj}`"
  )))
}

fn is_fn_like(v: &Value) -> bool {
  v.is_func() || v.is_closure() || v.is_method()
}
