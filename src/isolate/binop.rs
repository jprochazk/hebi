use std::ops::{Add, Div, Mul, Rem, Sub};

use syntax::ast;

use super::call::Args;
use super::Isolate;
use crate::value::object::frame::{Frame, OnReturn, Stack};
use crate::value::object::Str;
use crate::value::Value;
use crate::{Error, Result};

pub fn add(vm: &mut Isolate, lhs: Value, rhs: Value) -> Result<Value> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::int(lhs.add(rhs)));
    } else if let Some(rhs) = rhs.clone().to_float() {
      return Ok(Value::float((lhs as f64).add(rhs)));
    }
  } else if let Some(lhs) = lhs.clone().to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::float(lhs.add(rhs as f64)));
    } else if let Some(rhs) = rhs.clone().to_float() {
      return Ok(Value::float(lhs.add(rhs)));
    }
  }

  if let Some(lhs) = lhs.to_class_instance() {
    if let Some(rhs) = rhs.to_class_instance() {
      if let Some(meta_method) = lhs.meta_method(&ast::Meta::Add) {
        if lhs.id() == rhs.id() {
          vm.push_frame(Frame::with_stack(
            &vm.module_registry,
            meta_method.clone(),
            1,
            OnReturn::Yield,
            Stack::view(vm.stack(), vm.current_frame().frame_size),
          )?);
          vm.stack_mut()[0] = rhs.into();
          let result = vm.call_recurse(
            meta_method,
            Args::new(lhs.clone().into(), Some(vm.stack().slice(0..1)), None),
          );
          vm.pop_frame();
          let value = result?;
          if value
            .clone()
            .to_class_instance()
            .filter(|v| v.id() == lhs.id())
            .is_none()
          {
            return Err(Error::runtime(format!(
              "`add` meta method must return an instance of `{}`",
              lhs.name().as_str()
            )));
          }
          return Ok(value);
        }
      }
    }
  }

  // TODO: span + print types
  Err(Error::runtime("cannot add values `{lhs}` and `{rhs}`"))
}

pub fn sub(vm: &mut Isolate, lhs: Value, rhs: Value) -> Result<Value> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::int(lhs.sub(rhs)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float((lhs as f64).sub(rhs)));
    }
  } else if let Some(lhs) = lhs.to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::float(lhs.sub(rhs as f64)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float(lhs.sub(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::runtime("cannot subtract values `{lhs}` and `{rhs}`"))
}

pub fn mul(vm: &mut Isolate, lhs: Value, rhs: Value) -> Result<Value> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::int(lhs.mul(rhs)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float((lhs as f64).mul(rhs)));
    }
  } else if let Some(lhs) = lhs.to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::float(lhs.mul(rhs as f64)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float(lhs.mul(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::runtime("cannot multiply values `{lhs}` and `{rhs}`"))
}

pub fn div(vm: &mut Isolate, lhs: Value, rhs: Value) -> Result<Value> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      if rhs == 0 {
        return Ok(Value::float((lhs as f64).div(rhs as f64)));
      }
      return Ok(Value::int(lhs.div(rhs)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float((lhs as f64).div(rhs)));
    }
  } else if let Some(lhs) = lhs.to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      if rhs == 0 {
        return Ok(Value::float(lhs.div(rhs as f64)));
      }
      return Ok(Value::float(lhs.div(rhs as f64)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float(lhs.div(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::runtime("cannot divide values `{lhs}` and `{rhs}`"))
}

pub fn rem(vm: &mut Isolate, lhs: Value, rhs: Value) -> Result<Value> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      if rhs == 0 {
        return Ok(Value::float((lhs as f64).rem(rhs as f64)));
      }
      return Ok(Value::int(lhs.rem(rhs)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float((lhs as f64).rem(rhs)));
    }
  } else if let Some(lhs) = lhs.to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      if rhs == 0 {
        return Ok(Value::float(lhs.rem(rhs as f64)));
      }
      return Ok(Value::float(lhs.rem(rhs as f64)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float(lhs.rem(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::runtime("cannot divide values `{lhs}` and `{rhs}`"))
}

pub fn pow(vm: &mut Isolate, lhs: Value, rhs: Value) -> Result<Value> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      if rhs < 0 {
        let rhs = (-rhs) as u32;
        let denom = lhs.pow(rhs) as f64;
        return Ok(Value::float(1.0 / denom));
      } else {
        let rhs = rhs as u32;
        return Ok(Value::int(lhs.pow(rhs)));
      }
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float((lhs as f64).powf(rhs)));
    }
  } else if let Some(lhs) = lhs.to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(Value::float(lhs.powf(rhs as f64)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(Value::float(lhs.powf(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::runtime(
    "cannot exponentiate value `{lhs}` by `{rhs}`",
  ))
}
