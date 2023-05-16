use std::cell::Cell;
use std::fmt::Display;

use indexmap::IndexMap;

use super::ptr::{Any, Ptr};
use super::{Function, FunctionDescriptor, Object, String, Table};
use crate as hebi;
use crate::ctx::Context;
use crate::object;
use crate::value::Value;

#[derive(Debug)]
pub struct ClassInstance {
  pub name: Ptr<String>,
  pub fields: Ptr<Table>,
  pub parent: Option<Ptr<ClassType>>,
  pub is_frozen: Cell<bool>,
}

impl ClassInstance {
  pub fn new(cx: &Context, type_: &ClassType) -> Self {
    let name = type_.name.clone();
    let fields = cx.alloc(type_.fields.copy());
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
  fn type_name(&self) -> &'static str {
    "Instance"
  }

  fn named_field(
    &self,
    cx: &crate::ctx::Context,
    name: Ptr<String>,
  ) -> crate::Result<Option<crate::value::Value>> {
    let _ = cx;
    Ok(self.fields.get(&name))
  }

  fn set_named_field(
    &self,
    cx: &crate::ctx::Context,
    name: Ptr<String>,
    value: crate::value::Value,
  ) -> crate::Result<()> {
    if !self.is_frozen() {
      self.fields.insert(cx.alloc(String::owned(name)), value);
      return Ok(());
    }

    if !self.fields.set(&name, value) {
      hebi::fail!("cannot add field `{name}` to frozen class");
    }

    Ok(())
  }
}

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
  fn type_name(&self) -> &'static str {
    "Instance"
  }

  fn named_field(
    &self,
    cx: &crate::ctx::Context,
    name: Ptr<String>,
  ) -> crate::Result<Option<crate::value::Value>> {
    self.class.named_field(cx, name)
  }

  // TODO: delegate everything to `this`
}

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
    let name = if let Some(function) = self.function.clone_cast::<Function>() {
      function.descriptor.name.clone()
    } else {
      unreachable!();
    };

    write!(f, "<method `{}`>", name)
  }
}

impl Object for ClassMethod {
  fn type_name(&self) -> &'static str {
    "Method"
  }
}

#[derive(Debug)]
pub struct ClassType {
  pub name: Ptr<String>,
  pub init: Option<Ptr<Function>>,
  pub fields: Ptr<Table>,
  pub methods: IndexMap<Ptr<String>, Ptr<Function>>,
  pub parent: Option<Ptr<ClassType>>,
}

impl ClassType {
  pub fn new(
    name: Ptr<String>,
    init: Option<Ptr<Function>>,
    fields: Ptr<Table>,
    methods: IndexMap<Ptr<String>, Ptr<Function>>,
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
  fn type_name(&self) -> &'static str {
    "Class"
  }

  fn named_field(&self, cx: &hebi::ctx::Context, name: Ptr<String>) -> hebi::Result<Option<Value>> {
    let _ = cx;

    Ok(self.methods.get(&name).cloned().map(Value::object))
  }
}

#[derive(Debug)]
pub struct ClassDescriptor {
  pub name: Ptr<String>,
  pub methods: IndexMap<Ptr<String>, Ptr<FunctionDescriptor>>,
  pub fields: Ptr<Table>,
}

impl Display for ClassDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}` descriptor>", self.name)
  }
}

impl Object for ClassDescriptor {
  fn type_name(&self) -> &'static str {
    "ClassDescriptor"
  }
}
