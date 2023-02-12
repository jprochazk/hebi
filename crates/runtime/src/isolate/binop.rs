use std::ops::{Add, Div, Mul, Rem, Sub};

use super::*;

pub fn add(lhs: Value, rhs: Value) -> Result<Value, Error> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::int(lhs.add(rhs)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float((lhs as f64).add(rhs)));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::float(lhs.add(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float(lhs.add(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::new(concat!("cannot add values")))
}

pub fn sub(lhs: Value, rhs: Value) -> Result<Value, Error> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::int(lhs.sub(rhs)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float((lhs as f64).sub(rhs)));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::float(lhs.sub(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float(lhs.sub(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::new(concat!("cannot subtract values")))
}

pub fn mul(lhs: Value, rhs: Value) -> Result<Value, Error> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::int(lhs.mul(rhs)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float((lhs as f64).mul(rhs)));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(Value::float(lhs.mul(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float(lhs.mul(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::new(concat!("cannot multiply values")))
}

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

  // TODO: span + print types
  Err(Error::new("cannot divide values"))
}

pub fn rem(lhs: Value, rhs: Value) -> Result<Value, Error> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      if rhs == 0 {
        return Ok(Value::float((lhs as f64).rem(rhs as f64)));
      }
      return Ok(Value::int(lhs.rem(rhs)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float((lhs as f64).rem(rhs)));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      if rhs == 0 {
        return Ok(Value::float(lhs.rem(rhs as f64)));
      }
      return Ok(Value::float(lhs.rem(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(Value::float(lhs.rem(rhs)));
    }
  }

  // TODO: span + print types
  Err(Error::new("cannot divide values"))
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

  // TODO: span + print types
  Err(Error::new("cannot exponentiate value"))
}
