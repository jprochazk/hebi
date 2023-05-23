use std::fmt::{Debug, Display};

use super::{Object, Ptr};
use crate::value::Value;
use crate::{Result, Scope};

pub type BuiltinCallback = fn(Value, Scope<'_>) -> Result<Value>;
pub type TypedBuiltinCallback<T> = fn(Ptr<T>, Scope<'_>) -> Result<Value>;

#[derive(Debug)]
pub struct BuiltinMethod {
  this: Value,
  function: BuiltinCallback,
}

impl BuiltinMethod {
  /// # Safety
  /// - type of `this` must match expected type of `function` first param
  ///
  /// Easiest way to ensure the safety invariant is to use the
  /// `builtin_callback` macro to create the callback.
  pub unsafe fn new(this: Value, function: BuiltinCallback) -> Self {
    Self { this, function }
  }

  pub fn call(&self, scope: Scope<'_>) -> Result<Value> {
    (self.function)(self.this.clone(), scope)
  }
}

impl Display for BuiltinMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<builtin method>")
  }
}

impl Object for BuiltinMethod {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Unknown"
  }
}

declare_object_type!(BuiltinMethod);

macro_rules! builtin_callback {
  ($function:expr) => {{
    let cb: $crate::object::builtin::BuiltinCallback =
      |this: $crate::value::Value, scope: $crate::Scope<'_>| {
        let this = unsafe { this.to_object_unchecked::<Self>() };
        let function: $crate::object::builtin::TypedBuiltinCallback<Self> = $function;
        function(this, scope)
      };
    cb
  }};
}
