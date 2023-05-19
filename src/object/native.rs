use std::any::{Any as StdAny, TypeId};
use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::string::String as StdString;
use std::sync::Arc;

use indexmap::IndexMap;

use super::{Object, Ptr, String};
use crate::value::Value;
use crate::{Result, Scope};

pub type Callback<R> = Arc<dyn Fn(Scope<'_>) -> R + Send + Sync + 'static>;
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
  fn type_name(&self) -> &'static str {
    "NativeClassInstance"
  }
}

#[derive(Debug)]
pub struct NativeClass {
  pub name: Ptr<String>,
  pub init: Option<Ptr<NativeFunction>>,
  pub fields: IndexMap<Ptr<String>, NativeField>,
  pub methods: IndexMap<Ptr<String>, NativeMethod>,
  pub static_methods: IndexMap<Ptr<String>, NativeMethod>,
}

#[derive(Debug)]
pub enum NativeMethod {
  Sync(Ptr<NativeFunction>),
  Async(Ptr<NativeAsyncFunction>),
}

#[derive(Debug)]
pub struct NativeField {
  pub get: Ptr<NativeFunction>,
  pub set: Ptr<NativeFunction>,
}

impl Display for NativeClass {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native class `{}`>", self.name)
  }
}

impl Object for NativeClass {
  fn type_name(&self) -> &'static str {
    "NativeClass"
  }
}

pub struct NativeClassDescriptor {
  pub(crate) name: StdString,
  pub(crate) type_id: TypeId,
  pub(crate) init: Option<SyncCallback>,
  pub(crate) fields: IndexMap<StdString, NativeFieldDescriptor>,
  pub(crate) methods: IndexMap<StdString, NativeMethodDescriptor>,
  pub(crate) static_methods: IndexMap<StdString, NativeMethodDescriptor>,
}

pub struct NativeFieldDescriptor {
  pub get: SyncCallback,
  pub set: Option<SyncCallback>,
}

pub enum NativeMethodDescriptor {
  Sync(SyncCallback),
  Async(AsyncCallback),
}
