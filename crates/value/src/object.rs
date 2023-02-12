#[macro_use]
mod macros;
pub mod class;
pub mod dict;
pub mod error;
#[doc(hidden)]
pub mod frame;
pub mod func;
pub mod handle;
pub mod module;

use std::hash::Hash;

use beef::lean::Cow;
use indexmap::Equivalent;
use paste::paste;

use crate::ptr::{Ref, RefMut};
use crate::util::join::JoinIter;
use crate::{Ptr, Value};

pub type String = std::string::String;
pub type List = std::vec::Vec<Value>;
pub use class::{Class, ClassDef, ClassDesc, Method, Proxy};
pub use dict::Dict;
pub use error::Error;
use frame::Frame;
pub use func::{Closure, ClosureDesc, Func};
pub use module::{Module, Path, Registry};

use self::dict::Key;

// TODO: force all in `Repr` to implement this

pub trait Access {
  fn get<Q>(&self, key: &Q) -> Result<Option<&Value>, Error>
  where
    Q: Equivalent<Key> + Hash;

  fn set<Q>(&mut self, key: &Q, value: Value) -> Result<(), Error>
  where
    Q: Equivalent<Key> + Hash;
}

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

impl PartialEq for Object {
  fn eq(&self, other: &Self) -> bool {
    match (&self.repr, &other.repr) {
      (Repr::String(a), Repr::String(b)) => a == b,
      (Repr::List(a), Repr::List(b)) => a == b,
      (Repr::Dict(a), Repr::Dict(b)) => a == b,
      (a, b) => std::ptr::eq(a, b),
    }
  }
}
impl Eq for Object {}

impl Hash for Object {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match &self.repr {
      Repr::String(v) => v.hash(state),
      Repr::List(v) => v.hash(state),
      v => hash_ptr(v, state),
    }
  }
}

fn hash_ptr<T, H: std::hash::Hasher>(v: &T, state: &mut H) {
  (v as *const _ as usize).hash(state)
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
      Repr::String(v) => write!(f, "\"{v}\""),
      Repr::List(v) => f.debug_list().entries(v.iter().map(unit)).finish(),
      Repr::Dict(v) => f.debug_map().entries(v.iter().map(tuple2)).finish(),
      Repr::Func(v) => write!(f, "<func {}>", v.name),
      Repr::Closure(v) => write!(f, "<closure {}>", v.desc.borrow().func.name),
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

pub trait ObjectHandle: private::Sealed + Sized {
  fn as_self(o: &Ptr<Object>) -> Option<Ref<'_, Self>>;
  fn as_self_mut(o: &mut Ptr<Object>) -> Option<RefMut<'_, Self>>;
  fn is_self(o: &Ptr<Object>) -> bool;
}

mod private {
  pub trait Sealed {}
}

#[cfg(test)]
mod tests;
