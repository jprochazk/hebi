use std::fmt::Display;

use indexmap::IndexMap;

use super::ptr::Ptr;
use super::BoundFunction;
use super::ReturnAddr;
use super::{Function, FunctionDescriptor, Object, Str, Table};
use crate as hebi;
use crate::value::Value;
use crate::vm::global::Global;
use crate::vm::thread::CallResult;
use crate::{Result, Scope};

#[derive(Debug)]
pub struct ClassInstance {
  pub name: Ptr<Str>,
  pub fields: Ptr<Table>,
  pub parent: Option<Ptr<ClassType>>,
}

impl ClassInstance {
  pub fn new(global: Global, type_: &ClassType) -> Self {
    let name = type_.name.clone();
    let fields = global.alloc(type_.fields.copy());
    for (key, method) in type_.methods.iter() {
      fields.insert(key.clone(), Value::object(method.clone()));
    }
    let parent = type_.parent.clone();
    Self {
      name,
      fields,
      parent,
    }
  }
}

impl Display for ClassInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}` instance>", self.name)
  }
}

impl Object for ClassInstance {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Instance"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    let value = this
      .fields
      .get(&name)
      .ok_or_else(|| error!("`{this}` has no field `{name}`"))?;

    // bind functions
    if let Some(function) = value.clone().to_object::<Function>() {
      return Ok(Value::object(
        scope.alloc(BoundFunction::new(this.into_any(), function)),
      ));
    }

    Ok(value)
  }

  fn named_field_opt(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    let value = this.fields.get(&name);

    // bind functions
    if let Some(value) = value.clone() {
      if let Some(function) = value.to_object::<Function>() {
        return Ok(Some(Value::object(
          scope.alloc(BoundFunction::new(this.into_any(), function)),
        )));
      }
    }

    Ok(value)
  }

  fn set_named_field(_: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>, value: Value) -> Result<()> {
    if !this.fields.set(&name, value) {
      fail!("`{this}` has no field `{name}`");
    }

    Ok(())
  }
}
declare_object_type!(ClassInstance);

#[derive(Debug)]
pub struct ClassProxy {
  pub this: Ptr<ClassInstance>,
  pub class: Ptr<ClassType>,
}

impl Display for ClassProxy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}` instance>", self.this.name)
  }
}

impl Object for ClassProxy {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Instance"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    /*
    class Foo(Bar):
      fn test(self):
        super is Foo ?
        super is Bar ?
    */
    todo!()
  }

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    let method = this
      .class
      .methods
      .get(name.as_str())
      .cloned()
      .ok_or_else(|| error!("failed to get field `{name}`"))?;

    Ok(Value::object(
      scope.alloc(BoundFunction::new(this.into_any(), method)),
    ))
  }

  fn named_field_opt(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    let method = this
      .class
      .methods
      .get(name.as_str())
      .cloned()
      .map(|method| scope.alloc(BoundFunction::new(this.into_any(), method)))
      .map(Value::object);

    Ok(method)
  }

  fn call(scope: Scope<'_>, this: Ptr<Self>, return_addr: ReturnAddr) -> Result<CallResult> {
    if let Some(init) = this.class.init.clone() {
      let init = scope.alloc(BoundFunction::new(this.into_any(), init));
      <BoundFunction as Object>::call(scope, init, return_addr)
    } else {
      Ok(CallResult::Return(Value::none()))
    }
  }

  // TODO: delegate everything to `this`
}

declare_object_type!(ClassProxy);

#[derive(Debug)]
pub struct ClassType {
  pub name: Ptr<Str>,
  pub init: Option<Ptr<Function>>,
  pub fields: Ptr<Table>,
  pub methods: IndexMap<Ptr<Str>, Ptr<Function>>,
  pub parent: Option<Ptr<ClassType>>,
}

impl ClassType {
  pub fn new(
    name: Ptr<Str>,
    init: Option<Ptr<Function>>,
    fields: Ptr<Table>,
    methods: IndexMap<Ptr<Str>, Ptr<Function>>,
    parent: Option<Ptr<ClassType>>,
  ) -> Self {
    Self {
      name,
      init,
      fields,
      methods,
      parent,
    }
  }
}

impl Display for ClassType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}`>", self.name)
  }
}

impl Object for ClassType {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Class"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    // A is Type == true
    // otherwise == false
    todo!()
  }

  fn named_field(_: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> hebi::Result<Value> {
    let value = this
      .methods
      .get(&name)
      .cloned()
      .map(Value::object)
      .ok_or_else(|| error!("failed to get field `{name}`"))?;
    Ok(value)
  }

  fn named_field_opt(_: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    let value = this.methods.get(&name).cloned().map(Value::object);
    Ok(value)
  }

  fn call(scope: Scope<'_>, this: Ptr<Self>, return_addr: ReturnAddr) -> Result<CallResult> {
    let instance = scope.alloc(ClassInstance::new(
      scope.thread.global.clone(),
      this.as_ref(),
    ));

    match this.init.as_ref() {
      Some(init) => {
        let init = scope.alloc(BoundFunction::new(instance.into_any(), init.clone()));
        <BoundFunction as Object>::call(scope, init, return_addr)
      }
      None => Ok(CallResult::Return(Value::object(instance))),
    }
  }
}

declare_object_type!(ClassType);

#[derive(Debug)]
pub struct ClassDescriptor {
  pub name: Ptr<Str>,
  pub init: Option<Ptr<FunctionDescriptor>>,
  pub methods: IndexMap<Ptr<Str>, Ptr<FunctionDescriptor>>,
  pub fields: Ptr<Table>,
}

impl Display for ClassDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}` descriptor>", self.name)
  }
}

impl Object for ClassDescriptor {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "ClassDescriptor"
  }

  default_instance_of!();
}

declare_object_type!(ClassDescriptor);
