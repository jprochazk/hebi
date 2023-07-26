use core::fmt::{Debug, Display};

use super::Value;
use crate::gc::Ref;
use crate::obj::func::FunctionProto;
use crate::obj::list::ListProto;
use crate::obj::map::MapProto;
use crate::obj::string::Str;
use crate::obj::tuple::TupleProto;
use crate::op::Offset;

#[derive(Clone, Copy)]
pub enum Constant {
  Float(NFloat),
  Int(i32),
  Offset(Offset<u64>),
  Str(Ref<Str>),
  Table(Ref<MapProto>),
  List(Ref<ListProto>),
  Tuple(Ref<TupleProto>),
  Func(Ref<FunctionProto>),
}

impl Constant {
  #[inline]
  pub fn value(&self) -> Value {
    match *self {
      Constant::Float(v) => Value::new(v.value()),
      Constant::Int(v) => Value::new(v),
      Constant::Str(v) => Value::new(v),
      Constant::Offset(_)
      | Constant::Table(_)
      | Constant::List(_)
      | Constant::Tuple(_)
      | Constant::Func(_) => unreachable!(),
    }
  }

  #[inline]
  pub fn offset(self) -> Offset<usize> {
    match self {
      Constant::Offset(offset) => Offset(offset.0 as usize),
      _ => unreachable!(),
    }
  }

  #[inline]
  pub fn function(self) -> Ref<FunctionProto> {
    match self {
      Constant::Func(proto) => proto,
      _ => unreachable!(),
    }
  }
}

impl From<NFloat> for Constant {
  fn from(value: NFloat) -> Self {
    Self::Float(value)
  }
}

impl From<i32> for Constant {
  fn from(value: i32) -> Self {
    Self::Int(value)
  }
}

impl From<Offset<u64>> for Constant {
  fn from(value: Offset<u64>) -> Self {
    Self::Offset(value)
  }
}

impl From<Ref<Str>> for Constant {
  fn from(value: Ref<Str>) -> Self {
    Self::Str(value)
  }
}

impl From<Ref<MapProto>> for Constant {
  fn from(value: Ref<MapProto>) -> Self {
    Self::Table(value)
  }
}

impl From<Ref<ListProto>> for Constant {
  fn from(value: Ref<ListProto>) -> Self {
    Self::List(value)
  }
}

impl From<Ref<TupleProto>> for Constant {
  fn from(value: Ref<TupleProto>) -> Self {
    Self::Tuple(value)
  }
}

impl From<Ref<FunctionProto>> for Constant {
  fn from(value: Ref<FunctionProto>) -> Self {
    Self::Func(value)
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NFloat(u64);

impl NFloat {
  pub fn new(v: f64) -> Option<Self> {
    if v.is_nan() {
      None
    } else {
      Some(Self(v.to_bits()))
    }
  }

  /// # Safety
  /// `v` must not be nan
  pub unsafe fn new_unchecked(v: f64) -> Self {
    Self(v.to_bits())
  }

  pub fn value(self) -> f64 {
    f64::from_bits(self.0)
  }
}

impl From<NFloat> for f64 {
  fn from(v: NFloat) -> Self {
    v.value()
  }
}

impl TryFrom<f64> for NFloat {
  type Error = ();

  fn try_from(value: f64) -> Result<Self, Self::Error> {
    NFloat::new(value).ok_or(())
  }
}

impl Display for NFloat {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Display::fmt(&self.value(), f)
  }
}

impl Debug for NFloat {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Debug::fmt(&self.value(), f)
  }
}

impl Display for Constant {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Constant::Float(v) => Display::fmt(v, f),
      Constant::Int(v) => Display::fmt(v, f),
      Constant::Offset(v) => Display::fmt(v, f),
      Constant::Str(v) => Display::fmt(v, f),
      Constant::Table(v) => Display::fmt(v, f),
      Constant::List(v) => Display::fmt(v, f),
      Constant::Tuple(v) => Display::fmt(v, f),
      Constant::Func(v) => Display::fmt(v, f),
    }
  }
}

impl Debug for Constant {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
      Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
      Self::Offset(arg0) => f.debug_tuple("Offset").field(arg0).finish(),
      Self::Str(arg0) => Debug::fmt(arg0, f),
      Self::Table(arg0) => Debug::fmt(arg0, f),
      Self::List(arg0) => Debug::fmt(arg0, f),
      Self::Tuple(arg0) => Debug::fmt(arg0, f),
      Self::Func(arg0) => Debug::fmt(arg0, f),
    }
  }
}
