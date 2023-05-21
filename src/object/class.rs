use std::cell::Cell;
use std::fmt::Display;

use indexmap::IndexMap;

use super::ptr::{Any, Ptr};
use super::{Function, FunctionDescriptor, Object, String, Table};
use crate as hebi;
use crate::value::Value;
use crate::vm::global::Global;
use crate::{object, Scope};

#[derive(Debug)]
pub struct ClassInstance {
  pub name: Ptr<String>,
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

  fn named_field(
    this: Ptr<Self>,
    _: Scope<'_>,
    name: Ptr<String>,
  ) -> crate::Result<Option<crate::value::Value>> {
    Ok(this.fields.get(&name))
  }

  fn set_named_field(
    this: Ptr<Self>,
    scope: Scope<'_>,
    name: Ptr<String>,
    value: crate::value::Value,
  ) -> crate::Result<()> {
    if !this.is_frozen() {
      this.fields.insert(scope.alloc(String::owned(name)), value);
      return Ok(());
    }

    if !this.fields.set(&name, value) {
      fail!("cannot add field `{name}` to frozen class");
    }

    Ok(())
  }
}
generate_vtable!(ClassInstance);

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

  fn named_field(
    this: Ptr<Self>,
    scope: Scope<'_>,
    name: Ptr<String>,
  ) -> crate::Result<Option<crate::value::Value>> {
    this.class.named_field(scope, name)
  }

  // TODO: delegate everything to `this`
}

generate_vtable!(ClassProxy);

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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Method"
  }
}

generate_vtable!(ClassMethod);

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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Class"
  }

  fn named_field(this: Ptr<Self>, _: Scope<'_>, name: Ptr<String>) -> hebi::Result<Option<Value>> {
    Ok(this.methods.get(&name).cloned().map(Value::object))
  }
}

generate_vtable!(ClassType);

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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "ClassDescriptor"
  }
}

generate_vtable!(ClassDescriptor);
