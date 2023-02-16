use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

use super::object::handle::Handle;
use super::object::{ClassDesc, ClosureDesc, Func, Path, Str};
use crate::Value;

#[derive(Clone)]
pub enum Constant {
  Str(Handle<Str>),
  Func(Handle<Func>),
  ClosureDesc(Handle<ClosureDesc>),
  ClassDesc(Handle<ClassDesc>),
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

impl From<Constant> for Value {
  fn from(value: Constant) -> Self {
    match value {
      Constant::Str(v) => Value::from(v),
      Constant::Func(v) => Value::from(v),
      Constant::ClosureDesc(v) => Value::from(v),
      Constant::ClassDesc(v) => Value::from(v),
      Constant::Path(v) => Value::from(v),
      Constant::Float(v) => Value::from(v.0),
    }
  }
}

macro_rules! impl_from {
  ($T:ident) => {
    impl From<$T> for Constant {
      fn from(value: $T) -> Self {
        Self::$T(Handle::new(value))
      }
    }

    impl From<Handle<$T>> for Constant {
      fn from(value: Handle<$T>) -> Self {
        Self::$T(value)
      }
    }
  };
}

impl_from!(Str);
impl_from!(Func);
impl_from!(ClosureDesc);
impl_from!(ClassDesc);
impl_from!(Path);

impl From<f64> for Constant {
  fn from(value: f64) -> Self {
    Self::Float(value.into())
  }
}

impl PartialEq for Constant {
  fn eq(&self, other: &Self) -> bool {
    use std::ops::Deref;
    match (self, other) {
      (Constant::Str(l), Constant::Str(r)) => l.borrow().deref() == r.borrow().deref(),
      (Constant::Func(l), Constant::Func(r)) => std::ptr::eq(&l.borrow(), &r.borrow()),
      (Constant::ClosureDesc(l), Constant::ClosureDesc(r)) => {
        std::ptr::eq(&l.borrow(), &r.borrow())
      }
      (Constant::ClassDesc(l), Constant::ClassDesc(r)) => std::ptr::eq(&l.borrow(), &r.borrow()),
      (Constant::Path(l), Constant::Path(r)) => l.borrow().segments() == r.borrow().segments(),
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
      Constant::Str(v) => v.borrow().as_str().hash(state),
      Constant::Func(v) => ptr_hash(v, state),
      Constant::ClosureDesc(v) => ptr_hash(v, state),
      Constant::ClassDesc(v) => ptr_hash(v, state),
      Constant::Path(v) => v.borrow().segments().hash(state),
      Constant::Float(v) => v.0.to_bits().hash(state),
    }
  }
}

fn ptr_hash<T, H: Hasher>(ptr: &T, state: &mut H) {
  (ptr as *const _ as usize).hash(state)
}

impl Debug for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Str(v) => Debug::fmt(v, f),
      Constant::Func(v) => Debug::fmt(v, f),
      Constant::ClosureDesc(v) => Debug::fmt(v, f),
      Constant::ClassDesc(v) => Debug::fmt(v, f),
      Constant::Path(v) => Debug::fmt(v, f),
      Constant::Float(v) => Debug::fmt(&v.0, f),
    }
  }
}

impl Display for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Str(v) => Display::fmt(v, f),
      Constant::Func(v) => Display::fmt(v, f),
      Constant::ClosureDesc(v) => Display::fmt(v, f),
      Constant::ClassDesc(v) => Display::fmt(v, f),
      Constant::Path(v) => Display::fmt(v, f),
      Constant::Float(v) => Display::fmt(&v.0, f),
    }
  }
}
