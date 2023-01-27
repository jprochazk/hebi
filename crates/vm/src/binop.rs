use std::ops::{Add, Div, Mul, Rem, Sub};

use super::*;

macro_rules! define {
  ($op:ident) => {
    pub fn $op(lhs: Value, rhs: Value) -> Result<Value, Error> {
      if let Some(lhs) = lhs.as_int() {
        if let Some(rhs) = rhs.as_int() {
          return Ok(Value::int(lhs.$op(rhs)));
        } else if let Some(rhs) = rhs.as_float() {
          return Ok(Value::float((lhs as f64).$op(rhs)));
        }
      } else if let Some(lhs) = lhs.as_float() {
        if let Some(rhs) = rhs.as_int() {
          return Ok(Value::float(lhs.$op(rhs as f64)));
        } else if let Some(rhs) = rhs.as_float() {
          return Ok(Value::float(lhs.$op(rhs)));
        }
      }

      // TODO: error message
      Err(Error)
    }
  };
}

define!(add);
define!(sub);
define!(mul);
define!(rem);

pub fn div(lhs: Value, rhs: Value) -> Result<Value, Error> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      if rhs == 0 {
        return Ok(Value::float((lhs as f64).div(rhs as f64)));
      }
      return Ok(Value::int(lhs.div(rhs)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float((lhs as f64).div(rhs)));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      if rhs == 0 {
        return Ok(Value::float(lhs.div(rhs as f64)));
      }
      return Ok(Value::float(lhs.div(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float(lhs.div(rhs)));
    }
  }

  // TODO: error message
  Err(Error)
}

pub fn pow(lhs: Value, rhs: Value) -> Result<Value, Error> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      return if rhs < 0 {
        let rhs = (-rhs) as u32;
        let denom = lhs.pow(rhs) as f64;
        Ok(Value::float(1.0 / denom))
      } else {
        let rhs = rhs as u32;
        Ok(Value::int(lhs.pow(rhs)))
      };
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float((lhs as f64).powf(rhs)));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::float(lhs.powf(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float(lhs.powf(rhs)));
    }
  }

  // TODO: error message
  Err(Error)
}
