pub mod class;
pub mod function;
pub mod list;
pub mod module;
pub mod string;
pub mod table;

pub(crate) mod ptr;

use std::any::Any as DynAny;
use std::fmt::{Debug, Display};

pub use class::{Class, ClassDescriptor};
pub use function::{Function, FunctionDescriptor};
pub use list::List;
pub use module::{Module, ModuleDescriptor};
pub use ptr::Ptr;
pub use string::String;
pub use table::Table;

use crate::ctx::Context;
use crate::span::SpannedError;
use crate::value::Value;
use crate::HebiResult;

pub type Any = Ptr<ptr::Any>;

pub trait Object: DynAny + Debug + Display {
  fn type_name(&self) -> &'static str;

  fn named_field(&self, cx: &Context, name: &str) -> HebiResult<Option<Value>> {
    let _ = cx;
    Err(SpannedError::new(format!("cannot get field `{name}`"), None).into())
  }

  fn set_named_field(&self, cx: &Context, name: &str, value: Value) -> HebiResult<()> {
    let _ = cx;
    let _ = value;
    Err(SpannedError::new(format!("cannot set field `{name}`"), None).into())
  }

  fn keyed_field(&self, cx: &Context, key: Value) -> HebiResult<Option<Value>> {
    let _ = cx;
    Err(SpannedError::new(format!("cannot get index `{key}`"), None).into())
  }

  fn set_keyed_field(&self, cx: &Context, key: Value, value: Value) -> HebiResult<()> {
    let _ = cx;
    let _ = value;
    Err(SpannedError::new(format!("cannot set index `{key}`"), None).into())
  }
}
