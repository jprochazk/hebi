use std::fmt::{Debug, Display};

use super::{Object, Ptr, Str};
use crate::value::Value;
use crate::vm::global::Global;
use crate::vm::thread::util::is_truthy;
use crate::{Result, Scope, Unbind};

pub type Callback = fn(Scope<'_>) -> Result<Value>;
pub type MethodCallback = fn(Value, Scope<'_>) -> Result<Value>;
pub type TypedMethodCallback<T> = fn(Ptr<T>, Scope<'_>) -> Result<Value>;

pub struct BuiltinFunction {
  pub name: Ptr<Str>,
  function: Callback,
}

impl BuiltinFunction {
  pub fn new(name: Ptr<Str>, function: Callback) -> Self {
    Self { name, function }
  }

  pub fn call(&self, scope: Scope<'_>) -> Result<Value> {
    (self.function)(scope)
  }
}

impl Debug for BuiltinFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BuiltinFunction").finish()
  }
}

impl Display for BuiltinFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<builtin function>")
  }
}

impl Object for BuiltinFunction {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "BuiltinFunction"
  }
}

declare_object_type!(BuiltinFunction);

pub struct BuiltinMethod {
  this: Value,
  function: MethodCallback,
}

impl BuiltinMethod {
  /// # Safety
  /// - type of `this` must match expected type of `function` first param
  ///
  /// Easiest way to ensure the safety invariant is to use the
  /// `builtin_callback` macro to create the callback.
  pub unsafe fn new(this: Value, function: MethodCallback) -> Self {
    Self { this, function }
  }

  pub fn call(&self, scope: Scope<'_>) -> Result<Value> {
    (self.function)(self.this.clone(), scope)
  }
}

impl Debug for BuiltinMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BuiltinMethod").finish()
  }
}

impl Display for BuiltinMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<builtin method>")
  }
}

impl Object for BuiltinMethod {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "BuiltinMethod"
  }
}

declare_object_type!(BuiltinMethod);

macro_rules! builtin_method {
  ($function:expr) => {{
    let cb: $crate::object::builtin::MethodCallback =
      |this: $crate::value::Value, scope: $crate::Scope<'_>| {
        let this = unsafe { this.to_object_unchecked::<Self>() };
        let function: $crate::object::builtin::TypedMethodCallback<Self> = $function;
        function(this, scope)
      };
    cb
  }};
}

fn to_int(scope: Scope<'_>) -> Result<Value> {
  let value = scope.param::<crate::Value>(0)?.unbind();
  if value.is_int() {
    Ok(value)
  } else if value.is_float() {
    let value = unsafe { value.to_float_unchecked() };
    Ok(Value::int(value as i32))
  } else {
    fail!("cannot convert `{value}` to a float")
  }
}

fn to_float(scope: Scope<'_>) -> Result<Value> {
  let value = scope.param::<crate::Value>(0)?.unbind();
  if value.is_int() {
    let value = unsafe { value.to_int_unchecked() };
    Ok(Value::float(value as f64))
  } else if value.is_float() {
    Ok(value)
  } else {
    fail!("cannot convert `{value}` to a float")
  }
}

fn to_bool(scope: Scope<'_>) -> Result<Value> {
  let value = scope.param::<crate::Value>(0)?.unbind();
  let bool = is_truthy(value);
  Ok(Value::bool(bool))
}

fn to_str(scope: Scope<'_>) -> Result<Value> {
  let value = scope.param::<crate::Value>(0)?.unbind();
  if let Some(str) = value.clone().to_object::<Str>() {
    Ok(Value::object(str))
  } else {
    let str = scope.alloc(Str::owned(value));
    Ok(Value::object(str))
  }
}

fn type_of(scope: Scope<'_>) -> Result<Value> {
  let value = scope.param::<crate::Value>(0)?.unbind();

  if value.is_float() {
    Ok(Value::object(scope.intern("float")))
  } else if value.is_int() {
    Ok(Value::object(scope.intern("int")))
  } else if value.is_bool() {
    Ok(Value::object(scope.intern("bool")))
  } else if value.is_none() {
    Ok(Value::object(scope.intern("none")))
  } else {
    let object = unsafe { value.to_any_unchecked() };
    Ok(Value::object(scope.intern(object.type_name())))
  }
}

macro_rules! bind_builtin {
  ($global:ident, $builtin:ident) => {{
    let name = $global.intern(stringify!($builtin));
    $global.set(
      name.clone(),
      $crate::value::Value::object($global.alloc($crate::object::builtin::BuiltinFunction::new(
        name, $builtin,
      ))),
    )
  }};
}

pub fn register_builtin_functions(global: &Global) {
  bind_builtin!(global, to_int);
  bind_builtin!(global, to_float);
  bind_builtin!(global, to_bool);
  bind_builtin!(global, to_str);
  bind_builtin!(global, type_of);
}
