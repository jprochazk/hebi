use std::fmt::Display;
use std::hash::{Hash, Hasher};

use super::object::ptr::Ptr;
use super::object::{ClassDescriptor, FunctionDescriptor, String};

#[derive(Clone)]
pub enum Constant {
  Reserved,
  String(Ptr<String>),
  Function(Ptr<FunctionDescriptor>),
  Class(Ptr<ClassDescriptor>),
  Float(NonNaNFloat),
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Reserved => write!(f, "<empty>"),
      Constant::String(v) => Display::fmt(v, f),
      Constant::Function(v) => Display::fmt(v, f),
      Constant::Class(v) => Display::fmt(v, f),
      Constant::Float(v) => Display::fmt(&v.0, f),
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
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

impl Eq for NonNaNFloat {}
