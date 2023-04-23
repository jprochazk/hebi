use std::fmt::Display;
use std::hash::{Hash, Hasher};

use super::object::ptr::Ptr;
use super::object::{ClassDescriptor, FunctionDescriptor, String};

#[derive(Clone)]
pub enum Constant {
  String(Ptr<String>),
  Function(Ptr<FunctionDescriptor>),
  Class(Ptr<ClassDescriptor>),
  Float(NonNaNFloat),
}

impl Hash for Constant {
  fn hash<H: Hasher>(&self, state: &mut H) {
    core::mem::discriminant(self).hash(state);
    match self {
      Constant::String(v) => v.hash(state),
      Constant::Function(v) => v.ptr_hash(state),
      Constant::Class(v) => v.ptr_hash(state),
      Constant::Float(v) => v.hash(state),
    }
  }
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::String(v) => Display::fmt(v, f),
      Constant::Function(v) => Display::fmt(v, f),
      Constant::Class(v) => Display::fmt(v, f),
      Constant::Float(v) => Display::fmt(&v.0, f),
    }
  }
}

impl PartialEq for Constant {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::String(l0), Self::String(r0)) => l0 == r0,
      (Self::Function(l0), Self::Function(r0)) => l0.ptr_eq(r0),
      (Self::Class(l0), Self::Class(r0)) => l0.ptr_eq(r0),
      (Self::Float(l0), Self::Float(r0)) => l0 == r0,
      _ => false,
    }
  }
}

impl Eq for Constant {}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct NonNaNFloat(f64);
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
