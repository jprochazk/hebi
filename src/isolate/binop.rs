use std::ops::{Add, Div, Mul, Rem, Sub};

use syntax::ast;

use super::call::Args;
use super::Isolate;
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
      let parent = match (lhs.parent(), rhs.parent()) {
        (Some(lhs_p), Some(rhs_p)) if lhs_p.ptr_eq(&rhs_p) => Some(lhs_p),
        _ => None,
      };
      if let Some(parent) = parent {
        if let Some(m) = parent.meta_method(&ast::Meta::Add) {
          // TODO call it somehow idk
          vm.call_recurse(m, Args::new(lhs.clone().into(), vm.stack().slice()));
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
