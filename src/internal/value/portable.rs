use crate::object::ptr::{Any, Ptr};
use crate::object::Object;

pub enum Value {
  Float(f64),
  Int(i32),
  Bool(bool),
  None,
  Object(Ptr<Any>),
}

const QNAN: u64 = 0b01111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000;

impl Value {
  pub fn float(v: f64) -> Self {
    if v.to_bits() & QNAN == QNAN {
      panic!("cannot construct a Value from an f64 which is already a quiet NaN");
    }
    Self::Float(v)
  }

  pub fn int(v: i32) -> Self {
    Self::Int(v)
  }

  // 0b000000_00000000_01111111_00111001_00101000_00000000_00001101_00100000

  pub fn bool(v: bool) -> Self {
    Self::Bool(v)
  }

  pub fn none() -> Self {
    Self::None
  }

  pub fn object<T: Object>(ptr: Ptr<T>) -> Self {
    Self::Object(ptr.into_any())
  }
}

// Type checks
impl Value {
  #[inline]
  pub fn is_float(&self) -> bool {
    matches!(self, Self::Float(..))
  }

  #[inline]
  pub fn is_int(&self) -> bool {
    matches!(self, Self::Int(..))
  }

  #[inline]
  pub fn is_bool(&self) -> bool {
    matches!(self, Self::Bool(..))
  }

  #[inline]
  pub fn is_none(&self) -> bool {
    matches!(self, Self::None)
  }

  #[inline]
  pub fn is_object(&self) -> bool {
    matches!(self, Self::Object(..))
  }
}

impl Clone for Value {
  fn clone(&self) -> Self {
    match self {
      Self::Float(v) => Self::Float(v.clone()),
      Self::Int(v) => Self::Int(v.clone()),
      Self::Bool(v) => Self::Bool(v.clone()),
      Self::None => Self::None,
      Self::Object(v) => Self::Object(v.clone()),
    }
  }
}

// Owned conversions
impl Value {
  pub fn to_float(self) -> Option<f64> {
    match self {
      Value::Float(v) => Some(v),
      _ => None,
    }
  }

  pub fn to_int(self) -> Option<i32> {
    match self {
      Value::Int(v) => Some(v),
      _ => None,
    }
  }

  pub fn to_bool(self) -> Option<bool> {
    match self {
      Value::Bool(v) => Some(v),
      _ => None,
    }
  }

  pub fn to_none(self) -> Option<()> {
    match self {
      Value::None => Some(()),
      _ => None,
    }
  }

  pub fn to_object(self) -> Option<Ptr<Any>> {
    match self {
      Value::Object(v) => Some(v),
      _ => None,
    }
  }
}
