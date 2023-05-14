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
pub use ptr::Ptr;
pub use string::String;
pub use table::Table;

use crate as hebi;
use crate::ctx::Context;
use crate::value::Value;

pub type Any = Ptr<ptr::Any>;

pub trait Object: DynAny + Debug + Display {
  fn type_name(&self) -> &'static str;

  fn named_field(&self, cx: &Context, name: Ptr<String>) -> hebi::Result<Option<Value>> {
    let _ = cx;
    hebi::fail!("cannot get field `{name}`")
  }

  fn set_named_field(&self, cx: &Context, name: Ptr<String>, value: Value) -> hebi::Result<()> {
    let _ = cx;
    let _ = value;
    hebi::fail!("cannot set field `{name}`")
  }

  fn keyed_field(&self, cx: &Context, key: Value) -> hebi::Result<Option<Value>> {
    let _ = cx;
    hebi::fail!("cannot get index `{key}`")
  }

  fn set_keyed_field(&self, cx: &Context, key: Value, value: Value) -> hebi::Result<()> {
    let _ = cx;
    let _ = value;
    hebi::fail!("cannot set index `{key}`")
  }

  fn contains(&self, cx: &Context, item: Value) -> hebi::Result<bool> {
    let _ = cx;
    let _ = item;
    hebi::fail!("cannot get item `{item}`")
  }

  fn add(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support `+`")
  }

  fn subtract(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support `-`")
  }

  fn multiply(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support `*`")
  }

  fn divide(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support `/`")
  }

  fn remainder(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support `%`")
  }

  fn pow(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support `**`")
  }

  fn invert(&self, cx: &Context) -> hebi::Result<Value> {
    let _ = cx;
    hebi::fail!("`{self}` does not support unary `-`")
  }

  fn not(&self, cx: &Context) -> hebi::Result<Value> {
    let _ = cx;
    hebi::fail!("`{self}` does not support `!`")
  }

  fn cmp(&self, cx: &Context, other: Value) -> hebi::Result<Ordering> {
    let _ = cx;
    let _ = other;
    hebi::fail!("`{self}` does not support comparison")
  }
}

pub fn is_callable(v: &Any) -> bool {
  v.is::<function::Function>() || v.is::<class::ClassMethod>()
}

pub fn is_class(v: &Any) -> bool {
  v.is::<class::ClassInstance>() || v.is::<class::ClassProxy>()
}
