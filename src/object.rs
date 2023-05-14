pub mod class;
pub mod function;
pub mod list;
pub mod module;
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
use crate::span::SpannedError;
use crate::value::Value;

pub type Any = Ptr<ptr::Any>;

pub trait Object: DynAny + Debug + Display {
  fn type_name(&self) -> &'static str;

  fn named_field(&self, cx: &Context, name: &str) -> hebi::Result<Option<Value>> {
    let _ = cx;
    Err(SpannedError::new(format!("cannot get field `{name}`"), None).into())
  }

  fn set_named_field(&self, cx: &Context, name: &str, value: Value) -> hebi::Result<()> {
    let _ = cx;
    let _ = value;
    Err(SpannedError::new(format!("cannot set field `{name}`"), None).into())
  }

  fn keyed_field(&self, cx: &Context, key: Value) -> hebi::Result<Option<Value>> {
    let _ = cx;
    Err(SpannedError::new(format!("cannot get index `{key}`"), None).into())
  }

  fn set_keyed_field(&self, cx: &Context, key: Value, value: Value) -> hebi::Result<()> {
    let _ = cx;
    let _ = value;
    Err(SpannedError::new(format!("cannot set index `{key}`"), None).into())
  }

  fn contains(&self, cx: &Context, item: Value) -> hebi::Result<bool> {
    todo!()
  }

  fn add(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    todo!()
  }

  fn subtract(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    todo!()
  }

  fn multiply(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    todo!()
  }

  fn divide(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    todo!()
  }

  fn remainder(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    todo!()
  }

  fn pow(&self, cx: &Context, other: Value) -> hebi::Result<Value> {
    todo!()
  }

  fn invert(&self) -> hebi::Result<()> {
    todo!()
  }

  fn not(&self) -> hebi::Result<()> {
    todo!()
  }

  fn cmp(&self, cx: &Context, other: Value) -> hebi::Result<Ordering> {
    todo!()
  }

  fn type_eq(&self, cx: &Context, other: Value) -> hebi::Result<bool> {
    todo!()
  }
}
