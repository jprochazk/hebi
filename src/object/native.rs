use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::rc::Rc;

use super::{Object, Ptr, String};
use crate::ctx::Context;
use crate::public::value::ValueRef;
use crate::value::Value;
use crate::{AsyncScope, Result, Scope, Unbind};

pub type Callback = for<'cx> fn(&'cx Scope<'cx>) -> Result<ValueRef<'cx>>;

pub struct NativeFunction {
  pub name: Ptr<String>,
  pub cb: Callback,
}

impl NativeFunction {
  pub fn call(&self, scope: Scope<'_>) -> Result<Value> {
    Ok((self.cb)(&scope)?.unbind())
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

pub type LocalBoxFuture<'a, T> = Pin<Box<dyn core::future::Future<Output = T> + 'a>>;

pub type AsyncCallback = Rc<dyn Fn(AsyncScope, Context) -> LocalBoxFuture<'static, Result<Value>>>;

pub struct NativeAsyncFunction {
  pub name: Ptr<String>,
  pub cb: AsyncCallback,
}

impl NativeAsyncFunction {
  pub fn call(&self, scope: AsyncScope) -> LocalBoxFuture<'static, Result<Value>> {
    let cx = scope.thread.cx.clone();
    (self.cb)(scope, cx)
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
