use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::rc::Rc;

use super::{Object, Ptr, String};
use crate::value::Value;
use crate::{Result, Scope};

pub type Callback<R> = Rc<dyn Fn(Scope<'_>) -> R>;
pub type LocalBoxFuture<'a, T> = Pin<Box<dyn core::future::Future<Output = T> + 'a>>;

pub type SyncCallback = Callback<Result<Value>>;
pub type AsyncCallback = Callback<LocalBoxFuture<'static, Result<Value>>>;

pub struct NativeFunction {
  pub name: Ptr<String>,
  pub cb: SyncCallback,
}

impl NativeFunction {
  pub fn call(&self, scope: Scope) -> Result<Value> {
    (self.cb)(scope)
  }
}

impl Debug for NativeFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NativeFunction")
      .field("name", &self.name)
      .finish()
  }
}

impl Display for NativeFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native function `{}`>", self.name)
  }
}

impl Object for NativeFunction {
  fn type_name(&self) -> &'static str {
    "NativeFunction"
  }
}

pub struct NativeAsyncFunction {
  pub name: Ptr<String>,
  pub cb: AsyncCallback,
}

impl NativeAsyncFunction {
  pub fn call(&self, scope: Scope) -> LocalBoxFuture<'static, Result<Value>> {
    (self.cb)(scope)
  }
}

impl Debug for NativeAsyncFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NativeAsyncFunction")
      .field("name", &self.name)
      .finish()
  }
}

impl Display for NativeAsyncFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native async function `{}`>", self.name)
  }
}

impl Object for NativeAsyncFunction {
  fn type_name(&self) -> &'static str {
    "NativeAsyncFunction"
  }
}
