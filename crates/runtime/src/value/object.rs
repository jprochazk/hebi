#[macro_use]
mod macros;

pub mod class;
pub mod dict;
pub mod error;
#[doc(hidden)]
pub mod frame;
pub mod func;
pub mod handle;
pub mod list;
pub mod module;
pub mod string;

use std::fmt::Debug;

use beef::lean::Cow;
pub use class::{Class, ClassDef, ClassDesc, Method, Proxy};
pub use dict::Dict;
pub use error::Error;
use frame::Frame;
pub use func::{Closure, ClosureDesc, Func};
pub use list::List;
pub use module::{Module, Path, Registry};
pub use string::Str;

use self::dict::{Key, StaticKey};
use super::util::join::JoinIter;
use super::{Ptr, Value};

// TODO: force all in `Repr` to implement this

pub trait Access {
  fn is_frozen(&self) -> bool {
    true
  }

  fn should_bind_methods(&self) -> bool {
    true
  }

  /// Represents the `obj.key` operation.
  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    Err(crate::Error::new(format!("cannot get field `{key}`")))
  }

  /// Represents the `obj.key = value` operation.
  fn field_set(&mut self, key: StaticKey, _: Value) -> Result<(), crate::Error> {
    Err(crate::Error::new(format!("cannot set field `{key}`")))
  }

  /// Represents the `obj[key]` operation.
  fn index_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    match key {
      Key::Int(_) => Ok(None),
      Key::Str(_) => self.field_get(key),
      Key::Ref(_) => self.field_get(key),
    }
  }

  /// Represents the `obj[key] = value` operation.
  fn index_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::Error> {
    match &key {
      Key::Int(_) => Err(crate::Error::new(format!("cannot set index `{key}`"))),
      Key::Str(_) => self.field_set(key, value),
      Key::Ref(_) => self.field_set(key, value),
    }
  }
}

#[derive(Clone)]
pub struct Object {
  repr: Repr,
}

object_repr! {
  enum Repr {
    Str,
    List,
    Dict,
    Func,
    Closure,
    ClosureDesc,
    Class,
    ClassDef,
    ClassDesc,
    Method,
    Proxy,
    Module,
    Path,
    Registry,
    Frame,
    Error,
  }
}

impl<'a> From<&'a str> for Object {
  fn from(value: &'a str) -> Self {
    Object {
      repr: Repr::Str(value.to_string().into()),
    }
  }
}

impl<'a> From<Cow<'a, str>> for Object {
  fn from(value: Cow<'a, str>) -> Self {
    Object {
      repr: Repr::Str(value.to_string().into()),
    }
  }
}

impl std::fmt::Debug for Object {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(&self.repr, f)
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
      Repr::Str(v) => write!(f, "\"{v}\""),
      Repr::List(v) => f.debug_list().entries(v.iter().map(unit)).finish(),
      Repr::Dict(v) => f.debug_map().entries(v.iter().map(tuple2)).finish(),
      Repr::Func(v) => write!(f, "<func {}>", v.name),
      Repr::Closure(v) => write!(f, "<closure {}>", v.desc.func.name),
      Repr::ClosureDesc(v) => write!(f, "<closure desc {} n={}>", v.func.name, v.num_captures),
      Repr::Class(v) => write!(f, "<class {}>", v.name),
      Repr::ClassDef(v) => write!(f, "<class def {}>", v.name),
      Repr::ClassDesc(v) => write!(f, "<class desc {}>", v.name),
      Repr::Method(v) => write!(f, "{}", v.func),
      Repr::Proxy(v) => write!(f, "{}", v.parent),
      Repr::Module(v) => write!(f, "<module {}>", v.name()),
      Repr::Path(v) => write!(f, "<path {}>", v.segments().iter().join(".")),
      Repr::Registry(_) => write!(f, "<registry>"),
      Repr::Frame(_) => write!(f, "<frame>"),
      Repr::Error(_) => write!(f, "<error>"),
    }
  }
}

pub trait ObjectHandle: Access + private::Sealed + Sized {
  fn as_self(o: &Ptr<Object>) -> Option<&Self>;
  fn as_self_mut(o: &mut Ptr<Object>) -> Option<&mut Self>;
  fn is_self(o: &Ptr<Object>) -> bool;
}

mod private {
  pub trait Sealed {}
}

#[cfg(test)]
mod tests;
