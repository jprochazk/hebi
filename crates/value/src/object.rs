#[macro_use]
mod macros;
pub mod func;

use std::cell::{Ref, RefMut};

use beef::lean::Cow;
use paste::paste;

use crate::Value;

pub type String = std::string::String;
pub type List = std::vec::Vec<Value>;
pub type Dict = indexmap::IndexMap<Value, Value>;
pub use func::Func;

#[derive(Clone)]
pub struct Object {
  repr: Repr,
}

object_repr! {
  enum Repr {
    String,
    List,
    Dict,
    Func,
  }
}

impl<'a> From<&'a str> for Object {
  fn from(value: &'a str) -> Self {
    Object {
      repr: Repr::String(value.to_string()),
    }
  }
}

impl<'a> From<Cow<'a, str>> for Object {
  fn from(value: Cow<'a, str>) -> Self {
    Object {
      repr: Repr::String(value.to_string()),
    }
  }
}

impl std::fmt::Debug for Object {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.repr {
      Repr::String(v) => write!(f, "{v:?}"),
      Repr::List(v) => f.debug_list().entries(v).finish(),
      Repr::Dict(v) => f.debug_map().entries(v).finish(),
      Repr::Func(v) => f
        .debug_struct("Func")
        .field("name", &v.name)
        .field("frame_size", &v.frame_size)
        .field("code.len", &v.code.len())
        .field("const.len", &v.const_pool.len())
        .field("params.min", &v.params.min)
        .field("params.max", &v.params.max)
        .field("kw.len", &v.params.kw.len())
        .finish(),
    }
  }
}

impl std::fmt::Display for Object {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    struct DebugAsDisplay<T: std::fmt::Display>(T);
    impl<T: std::fmt::Display> std::fmt::Debug for DebugAsDisplay<T> {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
      }
    }
    fn unit<T>(v: T) -> DebugAsDisplay<T>
    where
      T: std::fmt::Display,
    {
      DebugAsDisplay(v)
    }
    fn tuple2<A, B>((a, b): (A, B)) -> (DebugAsDisplay<A>, DebugAsDisplay<B>)
    where
      A: std::fmt::Display,
      B: std::fmt::Display,
    {
      (DebugAsDisplay(a), DebugAsDisplay(b))
    }

    match &self.repr {
      Repr::String(v) => write!(f, "\"{v}\""),
      Repr::List(v) => f.debug_list().entries(v.iter().map(unit)).finish(),
      Repr::Dict(v) => f.debug_map().entries(v.iter().map(tuple2)).finish(),
      Repr::Func(v) => write!(f, "<func {}>", v.name),
    }
  }
}

#[cfg(test)]
mod tests;
