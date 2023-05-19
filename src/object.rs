pub mod class;
pub mod function;
pub mod list;
pub mod module;
pub mod native;
pub mod string;
pub mod table;

pub(crate) mod ptr;

use std::any::Any as DynAny;
use std::cmp::Ordering;
use std::fmt::{Debug, Display};

pub use class::{ClassDescriptor, ClassType};
pub use function::{Function, FunctionDescriptor};
pub use list::List;
pub use module::{Module, ModuleDescriptor};
pub use ptr::{Any, Ptr};
pub use string::String;
pub use table::Table;

use self::class::{ClassInstance, ClassMethod, ClassProxy};
use self::native::{NativeAsyncFunction, NativeClassInstance, NativeFunction};
use crate as hebi;
use crate::value::Value;
use crate::Scope;

// TODO: impl this on Ptr<T> instead of T
pub trait Object: DynAny + Debug + Display {
  fn type_name(&self) -> &'static str;

  fn named_field(&self, scope: Scope<'_>, name: Ptr<String>) -> hebi::Result<Option<Value>> {
    let _ = scope;
    fail!("cannot get field `{name}`")
  }

  fn set_named_field(&self, scope: Scope<'_>, name: Ptr<String>, value: Value) -> hebi::Result<()> {
    let _ = scope;
    let _ = value;
    fail!("cannot set field `{name}`")
  }

  fn keyed_field(&self, scope: Scope<'_>, key: Value) -> hebi::Result<Option<Value>> {
    let _ = scope;
    fail!("cannot get index `{key}`")
  }

  fn set_keyed_field(&self, scope: Scope<'_>, key: Value, value: Value) -> hebi::Result<()> {
    let _ = scope;
    let _ = value;
    fail!("cannot set index `{key}`")
  }

  fn contains(&self, scope: Scope<'_>, item: Value) -> hebi::Result<bool> {
    let _ = scope;
    let _ = item;
    fail!("cannot get item `{item}`")
  }

  fn add(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Value> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support `+`")
  }

  fn subtract(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Value> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support `-`")
  }

  fn multiply(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Value> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support `*`")
  }

  fn divide(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Value> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support `/`")
  }

  fn remainder(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Value> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support `%`")
  }

  fn pow(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Value> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support `**`")
  }

  fn invert(&self, scope: Scope<'_>) -> hebi::Result<Value> {
    let _ = scope;
    fail!("`{self}` does not support unary `-`")
  }

  fn not(&self, scope: Scope<'_>) -> hebi::Result<Value> {
    let _ = scope;
    fail!("`{self}` does not support `!`")
  }

  fn cmp(&self, scope: Scope<'_>, other: Value) -> hebi::Result<Ordering> {
    let _ = scope;
    let _ = other;
    fail!("`{self}` does not support comparison")
  }
}

pub fn is_callable(v: &Ptr<Any>) -> bool {
  v.is::<Function>()
    || v.is::<ClassMethod>()
    || v.is::<NativeFunction>()
    || v.is::<NativeAsyncFunction>()
}

pub fn is_class(v: &Ptr<Any>) -> bool {
  v.is::<ClassInstance>() || v.is::<ClassProxy>() || v.is::<NativeClassInstance>()
}
