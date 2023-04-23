pub mod module;
pub(crate) mod ptr;

use std::any::Any as DynAny;
use std::fmt::{Debug, Display};

use crate::ctx::Context;
use crate::error::Result;
use crate::value::Value;

pub trait Object: DynAny + Debug + Display {
  fn name(&self) -> &'static str;
  fn get_field(&self, _: &Context, _: &str) -> Result<Option<Value>> {
    Ok(None)
  }
  fn set_field(&self, cx: &Context, key: &str, _: Value) -> Result<()> {
    Err(cx.error(format!("cannot set field `{key}`"), None))
  }
  /* fn get_index(&self, key: Value) -> Option<Value>;
  fn set_index(&self, key: Value, value: Value); */
}
