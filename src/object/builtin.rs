use std::fmt::{Debug, Display};

use indexmap::IndexMap;

use super::{List, Object, Ptr, Str};
use crate::object::{list, string};
use crate::value::Value;
use crate::vm::global::Global;
use crate::vm::thread::util::is_truthy;
use crate::{Bind, LocalBoxFuture, Result, Scope, Unbind};

pub type Callback = fn(Scope<'_>) -> Result<Value>;
pub type AsyncCallback = fn(Scope<'_>) -> LocalBoxFuture<'_, Result<Value>>;
pub type MethodCallback = fn(Value, Scope<'_>) -> Result<Value>;
pub type TypedMethodCallback<T> = fn(Ptr<T>, Scope<'_>) -> Result<Value>;

#[derive(Clone)]
pub struct BuiltinFunction {
  pub name: &'static str,
  function: Callback,
}

impl BuiltinFunction {
  pub fn new(name: &'static str, function: Callback) -> Self {
    Self { name, function }
  }

  pub fn call(&self, scope: Scope<'_>) -> Result<Value> {
    (self.function)(scope)
  }
}

impl Debug for BuiltinFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BuiltinFunction")
      .field("name", &self.name)
      .finish()
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

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }
}

declare_object_type!(BuiltinFunction);

pub struct BuiltinAsyncFunction {
  pub name: &'static str,
  function: AsyncCallback,
}

impl BuiltinAsyncFunction {
  pub fn new(name: &'static str, function: AsyncCallback) -> Self {
    Self { name, function }
  }

  pub fn call(&self, scope: Scope) -> LocalBoxFuture<'static, Result<Value>> {
    let scope = unsafe { ::core::mem::transmute::<Scope<'_>, Scope<'static>>(scope) };
    (self.function)(scope)
  }
}

impl Debug for BuiltinAsyncFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BuiltinAsyncFunction")
      .field("name", &self.name)
      .finish()
  }
}

impl Display for BuiltinAsyncFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<builtin function>")
  }
}

impl Object for BuiltinAsyncFunction {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "BuiltinAsyncFunction"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }
}

declare_object_type!(BuiltinAsyncFunction);

// pub struct BuiltinType {
//   // TODO: List, Str, Table, etc. globals
//   // TODO: special sentinel object type `Type` (also global)
// }
#[derive(Debug)]
pub struct BuiltinType {
  pub name: &'static str,
  methods: IndexMap<&'static str, BuiltinFunction>,
}

impl BuiltinType {
  pub fn builder(name: &'static str) -> BuiltinTypeBuilder {
    BuiltinTypeBuilder {
      name,
      methods: IndexMap::new(),
    }
  }
}

pub struct BuiltinTypeBuilder {
  name: &'static str,
  methods: IndexMap<&'static str, BuiltinFunction>,
}

impl BuiltinTypeBuilder {
  pub fn method(mut self, name: &'static str, f: Callback) -> Self {
    self.methods.insert(name, BuiltinFunction::new(name, f));
    self
  }

  pub fn finish(self) -> BuiltinType {
    BuiltinType {
      name: self.name,
      methods: self.methods,
    }
  }
}

macro_rules! builtin_type {
  ($name:ident { $($method_name:ident : $method_cb:expr),* }) => {
    $crate::object::builtin::BuiltinType::builder(stringify!($name))
      $(.method(stringify!($method_name), $method_cb))*
      .finish()
  }
}

impl Display for BuiltinType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<builtin type `{}`>", self.name)
  }
}

impl Object for BuiltinType {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "BuiltinType"
  }

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    Ok(
      this
        .named_field_opt(scope, name.clone())?
        .ok_or_else(|| error!("`{this}` has no field `{name}`"))?,
    )
  }

  fn named_field_opt(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    Ok(
      this
        .methods
        .get(name.as_str())
        .map(|method| Value::object(scope.alloc(method.clone()))),
    )
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }
}

declare_object_type!(BuiltinType);

#[derive(Clone)]
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

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
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

macro_rules! builtin_method_static {
  ($T:ident, $function:expr) => {{
    let cb: $crate::object::builtin::Callback = |mut scope: $crate::Scope<'_>| {
      use $crate::public::Unbind;
      let this = scope.param::<$crate::Value>(0)?;
      scope.consume_args(1);
      let this = match this.clone().unbind().to_object::<$T>() {
        Some(value) => value,
        None => fail!(
          "`{this}` is not an instance of {}",
          std::any::type_name::<$T>()
        ),
      };
      let function: $crate::object::builtin::TypedMethodCallback<$T> = $function;
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

async fn collect(mut scope: Scope<'_>) -> Result<Value> {
  let iterable = scope.param::<crate::Value>(0)?.unbind();

  let Some(iterable) = iterable.clone().to_any() else {
    fail!("`{iterable}` is not iterable");
  };

  let iter = iterable
    .named_field(scope.clone(), scope.intern("iter"))?
    .bind(scope.global());

  let iterator = scope.call(iter, &[]).await?.unbind();
  let Some(iterator) = iterator.clone().to_any() else {
    fail!("`{iterable}` is not an iterator");
  };

  let next = iterator
    .named_field(scope.clone(), scope.intern("next"))?
    .bind(scope.global());
  let done = iterator
    .named_field(scope.clone(), scope.intern("done"))?
    .bind(scope.global());

  let list = List::new();
  while !is_truthy(scope.call(done.clone(), &[]).await?.unbind()) {
    list.push(scope.call(next.clone(), &[]).await?.unbind());
  }
  let list = scope.alloc(list);

  Ok(Value::object(list))
}

macro_rules! bind_builtin_fn {
  ($global:ident, $builtin:ident) => {{
    let name = stringify!($builtin);
    $global.set(
      $global.intern(name),
      $crate::value::Value::object($global.alloc($crate::object::builtin::BuiltinFunction::new(
        name, $builtin,
      ))),
    )
  }};
  ($global:ident, async $builtin:ident) => {{
    let name = stringify!($builtin);
    $global.set(
      $global.intern(name),
      $crate::value::Value::object($global.alloc(
        $crate::object::builtin::BuiltinAsyncFunction::new(name, |scope| {
          Box::pin(($builtin)(scope))
        }),
      )),
    )
  }};
}

macro_rules! bind_builtin_type {
  ($global:ident, $builtin:expr) => {{
    let builtin = $builtin;
    $global.set(
      $global.intern(builtin.name),
      $crate::value::Value::object($global.alloc(builtin)),
    )
  }};
}

pub fn register_builtin_functions(global: &Global) {
  bind_builtin_fn!(global, to_int);
  bind_builtin_fn!(global, to_float);
  bind_builtin_fn!(global, to_bool);
  bind_builtin_fn!(global, to_str);
  bind_builtin_fn!(global, type_of);
  /* bind_builtin_fn!(global, async collect); */

  list::register_builtin_functions(global);
  string::register_builtin_functions(global);
}
