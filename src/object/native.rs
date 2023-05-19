use std::any::{Any as StdAny, TypeId};
use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::string::String as StdString;
use std::sync::Arc;

use indexmap::IndexMap;

use super::{Any, Object, Ptr, String};
use crate::value::Value;
use crate::vm::global::Global;
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

  fn named_field(&self, _: Scope<'_>, _: Ptr<String>) -> crate::Result<Option<Value>> {
    panic!("named field access on native class instances should go through the VM")
  }

  fn set_named_field(&self, _: Scope<'_>, _: Ptr<String>, _: Value) -> crate::Result<()> {
    panic!("named field access on native class instances should go through the VM")
  }
}

#[derive(Debug)]
pub struct NativeClass {
  pub name: Ptr<String>,
  pub type_id: TypeId,
  pub init: Option<Ptr<NativeFunction>>,
  pub fields: IndexMap<Ptr<String>, NativeField>,
  pub methods: IndexMap<Ptr<String>, NativeMethod>,
  pub static_methods: IndexMap<Ptr<String>, NativeMethod>,
}

impl NativeClass {
  pub fn new(global: Global, desc: &NativeClassDescriptor) -> Self {
    let name = global.alloc(String::owned(desc.name.clone()));

    let type_id = desc.type_id;

    let init = desc.init.clone().map(|init| {
      global.alloc(NativeFunction {
        name: global.intern("__init__"),
        cb: init,
      })
    });

    let mut fields = IndexMap::with_capacity(desc.fields.len());
    for (name, desc) in desc.fields.iter() {
      let name = global.alloc(String::owned(name.clone()));
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
      let name = global.alloc(String::owned(name.clone()));
      let method = NativeMethod::new(global.clone(), name.clone(), desc.clone());
      methods.insert(name, method);
    }

    let mut static_methods = IndexMap::with_capacity(desc.static_methods.len());
    for (name, desc) in desc.static_methods.iter() {
      let name = global.alloc(String::owned(name.clone()));
      let method = NativeMethod::new(global.clone(), name.clone(), desc.clone());
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
  fn type_name(&self) -> &'static str {
    "NativeClass"
  }

  fn named_field(&self, _: Scope<'_>, name: Ptr<String>) -> crate::Result<Option<Value>> {
    if let Some(method) = self.static_methods.get(name.as_str()) {
      Ok(Some(Value::object(method.to_object())))
    } else if let Some(method) = self.methods.get(name.as_str()) {
      Ok(Some(Value::object(method.to_object())))
    } else {
      Ok(None)
    }
  }
}

#[derive(Debug)]
pub enum NativeMethod {
  Sync(Ptr<NativeFunction>),
  Async(Ptr<NativeAsyncFunction>),
}

impl NativeMethod {
  pub fn new(global: Global, name: Ptr<String>, method: NativeMethodDescriptor) -> Self {
    match method {
      NativeMethodDescriptor::Sync(method) => Self::Sync(global.alloc(NativeFunction {
        name,
        cb: method.clone(),
      })),
      NativeMethodDescriptor::Async(method) => Self::Async(global.alloc(NativeAsyncFunction {
        name,
        cb: method.clone(),
      })),
    }
  }

  pub fn to_object(&self) -> Ptr<Any> {
    match self {
      NativeMethod::Sync(method) => method.clone().into_any(),
      NativeMethod::Async(method) => method.clone().into_any(),
    }
  }
}

#[derive(Debug)]
pub struct NativeField {
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
