use std::fmt::Display;

use indexmap::{IndexMap, IndexSet};

use super::function::Params;
use super::ptr::Ptr;
use super::{Function, Object, String, Table};
use crate::syntax::ast;

#[derive(Debug)]
pub struct Instance {
  pub name: Ptr<String>,
  pub meta: Ptr<Meta>,
  pub fields: Table,
  pub parent: Option<Ptr<Class>>,
  pub is_frozen: bool,
}

impl Display for Instance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}` instance>", self.name)
  }
}

impl Object for Instance {
  fn type_name(&self) -> &'static str {
    "Instance"
  }
}

#[derive(Debug)]
pub struct Class {
  pub descriptor: Ptr<ClassDescriptor>,
  pub meta: Ptr<Meta>,
  pub init: Option<Ptr<Function>>,
  pub methods: Table,
  pub fields: Table,
  pub parent: Option<Ptr<Class>>,
}

impl Display for Class {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class `{}`>", self.descriptor.name)
  }
}

impl Object for Class {
  fn type_name(&self) -> &'static str {
    "Class"
  }
}

#[derive(Debug)]
pub struct Meta {
  pub methods: IndexMap<ast::Meta, Ptr<Function>>,
}

#[derive(Debug)]
pub struct ClassDescriptor {
  pub name: Ptr<String>,
  pub params: Params,
  pub is_derived: bool,
  pub meta: IndexSet<ast::Meta>,
  pub methods: IndexSet<Ptr<String>>,
  pub fields: IndexSet<Ptr<String>>,
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
