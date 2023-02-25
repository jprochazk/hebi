use std::fmt::Display;
use std::hash::{Hash, Hasher};

use super::handle::Handle;
use super::object::{ClassDescriptor, FunctionDescriptor, Path, Str};
use super::Value;

pub enum Constant {
  Str(Handle<Str>),
  FunctionDescriptor(Handle<FunctionDescriptor>),
  ClassDescriptor(Handle<ClassDescriptor>),
  Path(Handle<Path>),
  Float(NonNaNFloat),
}

#[derive(Clone, Copy)]
pub struct NonNaNFloat(f64);
impl From<f64> for NonNaNFloat {
  fn from(value: f64) -> Self {
    if value.is_nan() {
      panic!("value is NaN")
    }
    Self(value)
  }
}

macro_rules! impl_from {
  ($T:ident) => {
    impl From<Handle<$T>> for Constant {
      fn from(value: Handle<$T>) -> Self {
        Self::$T(value)
      }
    }
  };
}

impl_from!(Str);
impl_from!(FunctionDescriptor);
impl_from!(ClassDescriptor);
impl_from!(Path);

impl From<f64> for Constant {
  fn from(value: f64) -> Self {
    Self::Float(value.into())
  }
}

impl From<Constant> for Value {
  fn from(value: Constant) -> Self {
    match value {
      Constant::Str(v) => Value::object(v),
      Constant::FunctionDescriptor(v) => Value::object(v),
      Constant::ClassDescriptor(v) => Value::object(v),
      Constant::Path(v) => Value::object(v),
      Constant::Float(v) => Value::float(v.0),
    }
  }
}

impl PartialEq for Constant {
  fn eq(&self, other: &Self) -> bool {
    use std::ops::Deref;
    match (self, other) {
      (Constant::Str(l), Constant::Str(r)) => l.deref() == r.deref(),
      (Constant::FunctionDescriptor(l), Constant::FunctionDescriptor(r)) => std::ptr::eq(&l, &r),
      (Constant::ClassDescriptor(l), Constant::ClassDescriptor(r)) => std::ptr::eq(&l, &r),
      (Constant::Path(l), Constant::Path(r)) => l.segments() == r.segments(),
      (Constant::Float(l), Constant::Float(r)) => l.0 == r.0,
      _ => false,
    }
  }
}

impl Eq for Constant {}

impl Hash for Constant {
  fn hash<H: Hasher>(&self, state: &mut H) {
    core::mem::discriminant(self).hash(state);
    match self {
      Constant::Str(v) => v.as_str().hash(state),
      Constant::FunctionDescriptor(v) => ptr_hash(v, state),
      Constant::ClassDescriptor(v) => ptr_hash(v, state),
      Constant::Path(v) => v.segments().hash(state),
      Constant::Float(v) => v.0.to_bits().hash(state),
    }
  }
}

fn ptr_hash<T, H: Hasher>(ptr: &T, state: &mut H) {
  (ptr as *const _ as usize).hash(state)
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let obj = match self {
      Constant::Str(v) => v.clone().widen(),
      Constant::FunctionDescriptor(v) => v.clone().widen(),
      Constant::ClassDescriptor(v) => v.clone().widen(),
      Constant::Path(v) => v.clone().widen(),
      Constant::Float(v) => return Display::fmt(&v.0, f),
    };
    Display::fmt(unsafe { obj._get() }, f)
  }
}

impl Clone for Constant {
  fn clone(&self) -> Self {
    match self {
      Self::Str(v) => Self::Str(v.clone()),
      Self::FunctionDescriptor(v) => Self::FunctionDescriptor(v.clone()),
      Self::ClassDescriptor(v) => Self::ClassDescriptor(v.clone()),
      Self::Path(v) => Self::Path(v.clone()),
      Self::Float(v) => Self::Float(*v),
    }
  }
}
