use std::fmt::Display;

use indexmap::IndexMap;

use super::ptr::Ptr;
use super::{Function, FunctionDescriptor, Object, String, Table};

#[derive(Debug)]
pub struct ClassInstance {
  pub name: Ptr<String>,
  pub fields: Ptr<Table>,
  pub parent: Option<Ptr<ClassType>>,
  pub is_frozen: bool,
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
}

#[derive(Debug)]
pub struct ClassType {
  pub name: Ptr<String>,
  pub init: Option<Ptr<Function>>,
  pub fields: Ptr<Table>,
  pub parent: Option<Ptr<ClassType>>,
}

impl ClassType {
  pub fn new(
    name: Ptr<String>,
    init: Option<Ptr<Function>>,
    fields: Ptr<Table>,
    parent: Option<Ptr<ClassType>>,
  ) -> Self {
    Self {
      name,
      init,
      fields,
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
