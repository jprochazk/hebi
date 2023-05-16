use std::fmt::{Debug, Display};

use super::{Object, Ptr, String};
use crate::public::value::ValueRef;
use crate::value::Value;
use crate::{Result, Scope, Unbind};

pub type Callback = for<'cx> fn(&'cx Scope<'cx>) -> Result<ValueRef<'cx>>;

pub struct NativeFunction {
  pub name: Ptr<String>,
  pub cb: Callback,
}

impl NativeFunction {
  pub fn call(&self, scope: Scope<'_>) -> Result<Value> {
    (self.cb)(&scope).map(|value| value.unbind())
  }
}

impl Debug for NativeFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NativeFunction").finish()
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
