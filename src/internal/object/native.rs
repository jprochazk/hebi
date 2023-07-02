use std::any::{Any as StdAny, TypeId};
use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::string::String as StdString;
use std::sync::Arc;

use indexmap::IndexMap;

use super::{Any, Object, Ptr, ReturnAddr, Str};
use crate::internal::error::Result;
use crate::internal::value::Value;
use crate::internal::vm::global::Global;
use crate::internal::vm::thread::{AsyncFrame, CallResult, Slot0};
use crate::public::Scope;

pub type LocalBoxFuture<'a, T> = Pin<Box<dyn core::future::Future<Output = T> + 'a>>;

pub type Callback<R> = Arc<dyn Fn(Scope<'_>) -> R + Send + Sync + 'static>;
pub type SyncCallback = Callback<Result<Value>>;
pub type AsyncCallback = Callback<LocalBoxFuture<'static, Result<Value>>>;

pub struct NativeFunction {
  pub name: Ptr<Str>,
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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "NativeFunction"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn call(scope: Scope<'_>, this: Ptr<Self>, _: ReturnAddr) -> Result<CallResult> {
    NativeFunction::call(this.as_ref(), scope).map(CallResult::Return)
  }
}

declare_object_type!(NativeFunction);

pub struct NativeAsyncFunction {
  pub name: Ptr<Str>,
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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "NativeAsyncFunction"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn call(scope: Scope<'_>, this: Ptr<Self>, _: ReturnAddr) -> Result<CallResult> {
    Ok(CallResult::Poll(AsyncFrame {
      stack_base: scope.stack_base,
      fut: NativeAsyncFunction::call(this.as_ref(), scope),
    }))
  }
}

declare_object_type!(NativeAsyncFunction);

#[derive(Debug)]
pub struct NativeClassInstance {
  pub instance: Box<dyn StdAny + Send>,
  pub class: Ptr<NativeClass>,
}

impl Display for NativeClassInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native class `{}` instance>", self.class.name)
  }
}

impl Object for NativeClassInstance {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "NativeClassInstance"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn named_field(mut scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    if let Some(getter) = this.class.fields.get(name.as_str()).map(|field| &field.get) {
      let scope = scope.enter_nested(
        Slot0::Receiver(Value::object(this.clone())),
        scope.args,
        None,
      );
      let result = NativeFunction::call(getter.as_ref(), scope.clone());
      scope.leave();
      result
    } else if let Some(method) = this.class.methods.get(name.as_str()) {
      Ok(Value::object(scope.alloc(NativeBoundFunction::new(
        this.clone(),
        method.clone(),
      ))))
    } else {
      fail!("`{this}` has no field `{name}`")
    }
  }

  fn named_field_opt(
    mut scope: Scope<'_>,
    this: Ptr<Self>,
    name: Ptr<Str>,
  ) -> Result<Option<Value>> {
    if let Some(getter) = this.class.fields.get(name.as_str()).map(|field| &field.get) {
      let scope = scope.enter_nested(
        Slot0::Receiver(Value::object(this.clone())),
        scope.args,
        None,
      );
      let result = NativeFunction::call(getter.as_ref(), scope.clone()).map(Some);
      scope.leave();
      result
    } else if let Some(method) = this.class.methods.get(name.as_str()) {
      Ok(Some(Value::object(scope.alloc(NativeBoundFunction::new(
        this.clone(),
        method.clone(),
      )))))
    } else {
      Ok(None)
    }
  }

  fn set_named_field(
    mut scope: Scope<'_>,
    this: Ptr<Self>,
    name: Ptr<Str>,
    value: Value,
  ) -> Result<()> {
    if let Some(setter) = this
      .class
      .fields
      .get(name.as_str())
      .and_then(|field| field.set.as_ref())
    {
      let args = scope.thread.push_args(&[value]);
      let scope = scope.enter_nested(Slot0::Receiver(Value::object(this.clone())), args, None);
      let result = NativeFunction::call(setter.as_ref(), scope.clone()).map(|_| ());
      scope.leave();
      result
    } else {
      fail!("`{this}` has no field `{name}`")
    }
  }
}

declare_object_type!(NativeClassInstance);

#[derive(Debug)]
pub struct NativeClass {
  pub name: Ptr<Str>,
  pub type_id: TypeId,
  pub init: Option<Ptr<NativeFunction>>,
  pub fields: IndexMap<Ptr<Str>, NativeField>,
  pub methods: IndexMap<Ptr<Str>, Ptr<Any>>,
  pub static_methods: IndexMap<Ptr<Str>, Ptr<Any>>,
}

impl NativeClass {
  pub fn new(global: Global, desc: &NativeClassDescriptor) -> Self {
    let name = global.alloc(Str::owned(desc.name.clone()));

    let type_id = desc.type_id;

    let init = desc.init.clone().map(|init| {
      global.alloc(NativeFunction {
        name: global.intern("__init__"),
        cb: init,
      })
    });

    let mut fields = IndexMap::with_capacity(desc.fields.len());
    for (name, desc) in desc.fields.iter() {
      let name = global.alloc(Str::owned(name.clone()));
      let field = NativeField {
        get: global.alloc(NativeFunction {
          name: global.intern("__get__"),
          cb: desc.get.clone(),
        }),
        set: desc.set.as_ref().map(|set| {
          global.alloc(NativeFunction {
            name: global.intern("__set__"),
            cb: set.clone(),
          })
        }),
      };
      fields.insert(name, field);
    }

    let mut methods = IndexMap::with_capacity(desc.methods.len());
    for (name, desc) in desc.methods.iter() {
      let name = global.alloc(Str::owned(name.clone()));
      let method = desc.to_function(name.clone(), &global);
      methods.insert(name, method);
    }

    let mut static_methods = IndexMap::with_capacity(desc.static_methods.len());
    for (name, desc) in desc.static_methods.iter() {
      let name = global.alloc(Str::owned(name.clone()));
      let method = desc.to_function(name.clone(), &global);
      static_methods.insert(name, method);
    }

    Self {
      name,
      type_id,
      init,
      fields,
      methods,
      static_methods,
    }
  }
}

impl Display for NativeClass {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native class `{}`>", self.name)
  }
}

impl Object for NativeClass {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "NativeClass"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn named_field(_: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    if let Some(method) = this.static_methods.get(name.as_str()) {
      Ok(Value::object(method.clone()))
    } else if let Some(method) = this.methods.get(name.as_str()) {
      Ok(Value::object(method.clone()))
    } else {
      fail!("failed to get field `{name}`")
    }
  }

  fn named_field_opt(_: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    if let Some(method) = this.static_methods.get(name.as_str()) {
      Ok(Some(Value::object(method.clone())))
    } else if let Some(method) = this.methods.get(name.as_str()) {
      Ok(Some(Value::object(method.clone())))
    } else {
      Ok(None)
    }
  }

  fn call(scope: Scope<'_>, this: Ptr<Self>, return_addr: ReturnAddr) -> Result<CallResult> {
    if let Some(init) = this.init.as_ref() {
      <NativeFunction as Object>::call(scope, init.clone(), return_addr)
    } else {
      fail!("native class `{}` has no initializer", this.name)
    }
  }
}

declare_object_type!(NativeClass);

#[derive(Debug)]
pub struct NativeBoundFunction {
  pub this: Ptr<NativeClassInstance>,
  pub function: Ptr<Any>, // NativeFunction or NativeAsyncFunction
}

impl NativeBoundFunction {
  fn new(this: Ptr<NativeClassInstance>, function: Ptr<Any>) -> Self {
    debug_assert!(function.is::<NativeFunction>() || function.is::<NativeAsyncFunction>());
    Self { this, function }
  }
}

impl Display for NativeBoundFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // TODO: name
    write!(f, "<native bound fn>")
  }
}

impl Object for NativeBoundFunction {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "NativeBoundFunction"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn call(mut scope: Scope<'_>, this: Ptr<Self>, _: ReturnAddr) -> Result<CallResult> {
    let scope = scope.enter_nested(
      Slot0::Receiver(Value::object(this.this.clone())),
      scope.args,
      None,
    );
    if this.function.is::<NativeFunction>() {
      let function = unsafe { this.function.clone().cast_unchecked::<NativeFunction>() };
      let result = NativeFunction::call(function.as_ref(), scope.clone()).map(CallResult::Return);
      scope.leave();
      result
    } else {
      // TODO: the outer scope is not left
      let function = unsafe {
        this
          .function
          .clone()
          .cast_unchecked::<NativeAsyncFunction>()
      };
      Ok(CallResult::Poll(AsyncFrame {
        stack_base: scope.stack_base,
        fut: NativeAsyncFunction::call(function.as_ref(), scope),
      }))
    }
  }
}

declare_object_type!(NativeBoundFunction);

#[derive(Debug)]
pub struct NativeField {
  // TODO: these probably don't need to be function objects
  pub get: Ptr<NativeFunction>,
  pub set: Option<Ptr<NativeFunction>>,
}

pub struct NativeClassDescriptor {
  pub(crate) name: StdString,
  pub(crate) type_id: TypeId,
  pub(crate) init: Option<SyncCallback>,
  pub(crate) fields: IndexMap<StdString, NativeFieldDescriptor>,
  pub(crate) methods: IndexMap<StdString, NativeMethodDescriptor>,
  pub(crate) static_methods: IndexMap<StdString, NativeMethodDescriptor>,
}

#[derive(Clone)]
pub struct NativeFieldDescriptor {
  pub get: SyncCallback,
  pub set: Option<SyncCallback>,
}

#[derive(Clone)]
pub enum NativeMethodDescriptor {
  Sync(SyncCallback),
  Async(AsyncCallback),
}

impl NativeMethodDescriptor {
  fn to_function(&self, name: Ptr<Str>, global: &Global) -> Ptr<Any> {
    match self {
      NativeMethodDescriptor::Sync(cb) => global
        .alloc(NativeFunction {
          name,
          cb: cb.clone(),
        })
        .into_any(),
      NativeMethodDescriptor::Async(cb) => global
        .alloc(NativeAsyncFunction {
          name,
          cb: cb.clone(),
        })
        .into_any(),
    }
  }
}
