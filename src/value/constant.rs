#![allow(clippy::wrong_self_convention)]

use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use super::Value;
use crate::bytecode::opcode as op;
use crate::object::ptr::Ptr;
use crate::object::{ClassDescriptor, FunctionDescriptor, String};

#[derive(Debug, Clone)]
pub enum Constant {
  Reserved,
  String(Ptr<String>),
  Function(Ptr<FunctionDescriptor>),
  Class(Ptr<ClassDescriptor>),
  Offset(op::Offset),
  Float(NonNaNFloat),
}

impl Constant {
  pub fn into_value(self) -> Value {
    match self {
      Constant::Reserved => {
        panic!("cannot access reserved constant pool slot")
      }
      Constant::String(v) => Value::object(v),
      Constant::Function(v) => Value::object(v),
      Constant::Class(v) => Value::object(v),
      Constant::Offset(_) => panic!("cannot convert constant jump offset to value"),
      Constant::Float(v) => Value::float(v.value()),
    }
  }
}

impl Constant {
  pub fn as_offset(&self) -> Option<&op::Offset> {
    if let Self::Offset(v) = self {
      Some(v)
    } else {
      None
    }
  }

  #[allow(dead_code)] // used in tests
  pub fn as_float(&self) -> Option<&NonNaNFloat> {
    if let Self::Float(v) = self {
      Some(v)
    } else {
      None
    }
  }
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Reserved => write!(f, "<empty>"),
      Constant::String(v) => Display::fmt(v, f),
      Constant::Function(v) => Display::fmt(v, f),
      Constant::Class(v) => Display::fmt(v, f),
      Constant::Offset(v) => Display::fmt(&v.0, f),
      Constant::Float(v) => Display::fmt(&v.0, f),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonNaNFloat(f64);

impl NonNaNFloat {
  pub fn value(&self) -> f64 {
    self.0
  }
}

impl Deref for NonNaNFloat {
  type Target = f64;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl From<NonNaNFloat> for f64 {
  fn from(value: NonNaNFloat) -> Self {
    value.0
  }
}

impl From<f64> for NonNaNFloat {
  fn from(value: f64) -> Self {
    if value.is_nan() {
      panic!("value is NaN")
    }
    Self(value)
  }
}

impl Hash for NonNaNFloat {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

impl Eq for NonNaNFloat {}
