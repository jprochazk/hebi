#![allow(clippy::needless_lifetimes)]

use crate as hebi;
use crate::value::Value;
use crate::vm::Scope;

pub trait FromHebi<'cx> {}
pub trait IntoHebi<'cx> {}

pub trait Function<'cx> {
  fn call(&self, cx: Scope<'cx>) -> hebi::Result<Value>;
}
