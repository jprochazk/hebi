#[macro_use]
mod macros;

#[macro_use]
pub mod access;

pub mod class;
pub mod dict;
pub mod error;
#[doc(hidden)]
pub mod frame;
pub mod func;
pub mod list;
pub mod module;
pub mod string;

use std::fmt::Display;

pub use access::Access;
use beef::lean::Cow;
pub use class::{Class, ClassDescriptor, ClassInstance, ClassSuperProxy, Method};
pub use dict::Dict;
pub use error::RuntimeError;
use frame::Frame;
pub use func::{Function, FunctionDescriptor};
pub use list::List;
pub use module::{Module, ModuleDescriptor, Path};
pub use string::Str;

use super::handle::Handle;
use super::{Ptr, Value};

pub struct Object {
  repr: Repr,
}

object_repr! {
  enum Repr {
    Str,
    List,
    Dict,
    Function,
    FunctionDescriptor,
    ClassInstance,
    Class,
    ClassDescriptor,
    Method,
    ClassSuperProxy,
    Module,
    ModuleDescriptor,
    Path,
    Frame,
    RuntimeError,
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

pub trait ObjectType: Access + private::Sealed + Sized + Into<Object> {
  fn as_ref(o: &Object) -> Option<&Self>;
  fn as_mut(o: &mut Object) -> Option<&mut Self>;
  fn is(o: &Object) -> bool;
}

impl Access for Ptr<Object> {
  fn is_frozen(&self) -> bool {
    unsafe { self._get() }.is_frozen()
  }

  fn should_bind_methods(&self) -> bool {
    unsafe { self._get() }.should_bind_methods()
  }

  fn field_get(&self, key: &str) -> crate::Result<Option<Value>> {
    unsafe { self._get() }.field_get(key)
  }

  fn field_set(&mut self, key: Handle<Str>, value: Value) -> crate::Result<()> {
    unsafe { self._get_mut() }.field_set(key, value)
  }

  fn index_get(&self, key: Value) -> crate::Result<Option<Value>> {
    unsafe { self._get() }.index_get(key)
  }

  fn index_set(&mut self, key: Value, value: Value) -> crate::Result<()> {
    unsafe { self._get_mut() }.index_set(key, value)
  }
}

mod private {
  pub trait Sealed {}
}
