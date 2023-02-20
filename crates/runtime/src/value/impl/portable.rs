use super::object::Object;
use super::*;

// TODO: make this indistinguishable from nanbox value

#[derive(Clone)]
pub enum Value {
  Float(f64),
  Int(i32),
  Bool(bool),
  None,
  Object(Ptr<object::Object>),
}

impl Value {
  pub fn float(v: f64) -> Self {
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

  pub fn object<T: ObjectType>(v: Handle<T>) -> Self {
    Self::Object(v.widen())
  }

  pub(crate) fn object_raw(v: Ptr<Object>) -> Self {
    Self::Object(v)
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

  pub fn to_object<T: ObjectType>(self) -> Option<Handle<T>> {
    self.to_object_raw().and_then(Handle::from_ptr)
  }

  pub(crate) fn to_object_raw(self) -> Option<Ptr<Object>> {
    match self {
      Value::Object(v) => Some(v),
      _ => None,
    }
  }

  pub(crate) fn as_object_raw(&self) -> Option<&Object> {
    match self {
      Value::Object(v) => Some(unsafe { v._get() }),
      _ => None,
    }
  }
}
