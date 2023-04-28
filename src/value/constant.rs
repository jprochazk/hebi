use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use super::object::ptr::Ptr;
use super::object::{ClassDescriptor, FunctionDescriptor, String};
use crate::bytecode::opcode as op;

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
  /// Returns `true` if the constant is [`Reserved`].
  ///
  /// [`Reserved`]: Constant::Reserved
  #[must_use]
  pub fn is_reserved(&self) -> bool {
    matches!(self, Self::Reserved)
  }

  /// Returns `true` if the constant is [`String`].
  ///
  /// [`String`]: Constant::String
  #[must_use]
  pub fn is_string(&self) -> bool {
    matches!(self, Self::String(..))
  }

  /// Returns `true` if the constant is [`Function`].
  ///
  /// [`Function`]: Constant::Function
  #[must_use]
  pub fn is_function(&self) -> bool {
    matches!(self, Self::Function(..))
  }

  /// Returns `true` if the constant is [`Class`].
  ///
  /// [`Class`]: Constant::Class
  #[must_use]
  pub fn is_class(&self) -> bool {
    matches!(self, Self::Class(..))
  }

  /// Returns `true` if the constant is [`Offset`].
  ///
  /// [`Offset`]: Constant::Offset
  #[must_use]
  pub fn is_offset(&self) -> bool {
    matches!(self, Self::Offset(..))
  }

  /// Returns `true` if the constant is [`Float`].
  ///
  /// [`Float`]: Constant::Float
  #[must_use]
  pub fn is_float(&self) -> bool {
    matches!(self, Self::Float(..))
  }

  pub fn as_string(&self) -> Option<&Ptr<String>> {
    if let Self::String(v) = self {
      Some(v)
    } else {
      None
    }
  }

  pub fn as_function(&self) -> Option<&Ptr<FunctionDescriptor>> {
    if let Self::Function(v) = self {
      Some(v)
    } else {
      None
    }
  }

  pub fn as_class(&self) -> Option<&Ptr<ClassDescriptor>> {
    if let Self::Class(v) = self {
      Some(v)
    } else {
      None
    }
  }

  pub fn as_offset(&self) -> Option<&op::Offset> {
    if let Self::Offset(v) = self {
      Some(v)
    } else {
      None
    }
  }

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
