use std::cell::Cell;
use std::fmt::Display;

use indexmap::IndexMap;

use super::builtin::BuiltinFunction;
use super::native::{NativeAsyncFunction, NativeFunction};
use super::ptr::{Any, Ptr};
use super::{Function, FunctionDescriptor, Object, Str, Table};
use crate as hebi;
use crate::value::Value;
use crate::vm::global::Global;
use crate::{object, Result, Scope};

#[derive(Debug)]
pub struct ClassInstance {
  pub name: Ptr<Str>,
  pub fields: Ptr<Table>,
  pub parent: Option<Ptr<ClassType>>,
  pub is_frozen: Cell<bool>,
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
      is_frozen: Cell::new(false),
    }
  }

  pub fn is_frozen(&self) -> bool {
    self.is_frozen.get()
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

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    let value = this
      .fields
      .get(&name)
      .ok_or_else(|| error!("`{this}` has no field `{name}`"))?;

    // bind functions
    if let Some(object) = value.clone().to_any() {
      if object::is_callable(&object) {
        return Ok(Value::object(
          scope.alloc(ClassMethod::new(this.into_any(), object)),
        ));
      }
    }

    Ok(value)
  }

  fn named_field_opt(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    let value = this.fields.get(&name);

    // bind functions
    if let Some(value) = value.clone() {
      if let Some(object) = value.to_any() {
        if object::is_callable(&object) {
          return Ok(Some(Value::object(
            scope.alloc(ClassMethod::new(this.into_any(), object)),
          )));
        }
      }
    }

    Ok(value)
  }

  fn set_named_field(
    scope: Scope<'_>,
    this: Ptr<Self>,
    name: Ptr<Str>,
    value: Value,
  ) -> Result<()> {
    if !this.is_frozen() {
      this.fields.insert(scope.alloc(Str::owned(name)), value);
      return Ok(());
    }

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

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    let method = this
      .class
      .methods
      .get(name.as_str())
      .cloned()
      .ok_or_else(|| error!("failed to get field `{name}`"))?;

    Ok(Value::object(
      scope.alloc(ClassMethod::new(this.into_any(), method.into_any())),
    ))
  }

  fn named_field_opt(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Option<Value>> {
    let method = this
      .class
      .methods
      .get(name.as_str())
      .cloned()
      .map(|method| scope.alloc(ClassMethod::new(this.into_any(), method.into_any())))
      .map(Value::object);

    Ok(method)
  }

  // TODO: delegate everything to `this`
}

declare_object_type!(ClassProxy);

// TODO: store name and type_name
#[derive(Debug)]
pub struct ClassMethod {
  this: Ptr<Any>,     // ClassInstance or ClassProxy
  function: Ptr<Any>, // Function or ???
}

impl ClassMethod {
  pub fn new(this: Ptr<Any>, function: Ptr<Any>) -> Self {
    assert!(object::is_class(&this));
    assert!(object::is_callable(&function));

    Self { this, function }
  }

  pub fn this(&self) -> Ptr<Any> {
    self.this.clone()
  }

  pub fn function(&self) -> Ptr<Any> {
    self.function.clone()
  }
}

impl Display for ClassMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if let Some(function) = self.function.clone_cast::<Function>() {
      write!(f, "<method `{}`>", function.descriptor.name)
    } else if let Some(function) = self.function.clone_cast::<BuiltinFunction>() {
      write!(f, "<method `{}`>", function.name)
    } else if let Some(function) = self.function.clone_cast::<NativeFunction>() {
      write!(f, "<method `{}`>", function.name)
    } else if let Some(function) = self.function.clone_cast::<NativeAsyncFunction>() {
      write!(f, "<method `{}`>", function.name)
    } else {
      write!(f, "<method>")
    }
  }
}

impl Object for ClassMethod {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Method"
  }
}

declare_object_type!(ClassMethod);

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
}

declare_object_type!(ClassType);

#[derive(Debug)]
pub struct ClassDescriptor {
  pub name: Ptr<Str>,
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
}

declare_object_type!(ClassDescriptor);
