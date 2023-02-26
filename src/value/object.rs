#[macro_use]
mod macros;

pub mod class;
pub mod dict;
pub mod error;
#[doc(hidden)]
pub mod frame;
pub mod func;
pub mod list;
pub mod module;
// pub mod native;
pub mod key;
pub mod string;

use std::fmt::Display;

use beef::lean::Cow;
pub use class::{Class, ClassDescriptor, ClassInstance, ClassSuperProxy, Method};
pub use dict::Dict;
pub use error::RuntimeError;
use frame::Frame;
pub use func::{Function, FunctionDescriptor};
pub use key::{Key, StaticKey};
pub use list::List;
pub use module::{Module, ModuleDescriptor, Path};
pub use string::Str;

use super::handle::Handle;
use super::{Ptr, Value};

pub trait Access {
  fn is_frozen(&self) -> bool {
    true
  }

  fn should_bind_methods(&self) -> bool {
    true
  }

  /// Represents the `obj.key` operation.
  fn field_get(&self, key: &Key<'_>) -> crate::Result<Option<Value>> {
    Err(crate::RuntimeError::script(
      format!("cannot get field `{key}`"),
      0..0,
    ))
  }

  /// Represents the `obj.key = value` operation.
  fn field_set(&mut self, key: StaticKey, value: Value) -> crate::Result<()> {
    drop(value);
    Err(crate::RuntimeError::script(
      format!("cannot set field `{key}`"),
      0..0,
    ))
  }

  /// Represents the `obj[key]` operation.
  fn index_get(&self, key: &Key<'_>) -> crate::Result<Option<Value>> {
    match key {
      Key::Int(_) => Ok(None),
      Key::Str(_) => self.field_get(key),
      Key::Ref(_) => self.field_get(key),
    }
  }

  /// Represents the `obj[key] = value` operation.
  fn index_set(&mut self, key: StaticKey, value: Value) -> crate::Result<()> {
    match &key {
      Key::Int(_) => Err(crate::RuntimeError::script(
        format!("cannot set index `{key}`"),
        0..0,
      )),
      Key::Str(_) => self.field_set(key, value),
      Key::Ref(_) => self.field_set(key, value),
    }
  }
}

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

  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::RuntimeError> {
    unsafe { self._get() }.field_get(key)
  }

  fn field_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::RuntimeError> {
    unsafe { self._get_mut() }.field_set(key, value)
  }

  fn index_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::RuntimeError> {
    unsafe { self._get() }.index_get(key)
  }

  fn index_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::RuntimeError> {
    unsafe { self._get_mut() }.index_set(key, value)
  }
}

mod private {
  pub trait Sealed {}
}
