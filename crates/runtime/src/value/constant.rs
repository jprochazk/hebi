use std::fmt::Display;
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
      Constant::Str(v) => Value::object(v),
      Constant::Func(v) => Value::object(v),
      Constant::ClosureDesc(v) => Value::object(v),
      Constant::ClassDesc(v) => Value::object(v),
      Constant::Path(v) => Value::object(v),
      Constant::Float(v) => Value::float(v.0),
    }
  }
}

macro_rules! impl_from {
  ($T:ident) => {
    impl From<$T> for Constant {
      fn from(value: $T) -> Self {
        Self::$T(Handle::alloc(value))
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
      (Constant::Str(l), Constant::Str(r)) => l.deref() == r.deref(),
      (Constant::Func(l), Constant::Func(r)) => std::ptr::eq(&l, &r),
      (Constant::ClosureDesc(l), Constant::ClosureDesc(r)) => std::ptr::eq(&l, &r),
      (Constant::ClassDesc(l), Constant::ClassDesc(r)) => std::ptr::eq(&l, &r),
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
      Constant::Func(v) => ptr_hash(v, state),
      Constant::ClosureDesc(v) => ptr_hash(v, state),
      Constant::ClassDesc(v) => ptr_hash(v, state),
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
      Constant::Func(v) => v.clone().widen(),
      Constant::ClosureDesc(v) => v.clone().widen(),
      Constant::ClassDesc(v) => v.clone().widen(),
      Constant::Path(v) => v.clone().widen(),
      Constant::Float(v) => return Display::fmt(&v.0, f),
    };
    Display::fmt(unsafe { obj._get() }, f)
  }
}
