#[macro_use]
mod macros;

#[macro_use]
pub mod access;

pub mod class;
pub mod dict;
#[doc(hidden)]
pub mod frame;
pub mod func;
pub mod list;
pub mod module;
pub mod native;
pub mod string;

pub use access::Access;
use beef::lean::Cow;
pub use class::{Class, ClassDescriptor, ClassInstance, ClassSuperProxy, Method};
pub use dict::Dict;
use frame::Frame;
pub use func::{Function, FunctionDescriptor};
pub use list::List;
pub use module::{Module, ModuleDescriptor, Path};
pub use native::{NativeClass, NativeClassInstance, NativeFunction, UserData};
pub use string::Str;

use super::handle::Handle;
use super::{Ptr, Value};
use crate::ctx::Context;
use crate::Error;

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
    Error,
    NativeFunction,
    NativeClass,
    UserData,
    NativeClassInstance,
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

pub trait ObjectType: Access + Sized + Into<Object> {
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

  fn field_get(&self, ctx: &Context, key: &str) -> crate::Result<Option<Value>> {
    unsafe { self._get() }.field_get(ctx, key)
  }

  fn field_set(&mut self, ctx: &Context, key: Handle<Str>, value: Value) -> crate::Result<()> {
    unsafe { self._get_mut() }.field_set(ctx, key, value)
  }

  fn index_get(&self, ctx: &Context, key: Value) -> crate::Result<Option<Value>> {
    unsafe { self._get() }.index_get(ctx, key)
  }

  fn index_set(&mut self, ctx: &Context, key: Value, value: Value) -> crate::Result<()> {
    unsafe { self._get_mut() }.index_set(ctx, key, value)
  }
}
