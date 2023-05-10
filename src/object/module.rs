use std::fmt::{Debug, Display};
use std::num::NonZeroU64;

use indexmap::{IndexMap, IndexSet};

use super::ptr::Ptr;
use super::{Function, FunctionDescriptor, Object, String, Table};
use crate::error::Result;

pub trait Loader {
  fn load(&self, path: &str) -> Result<&str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(Option<NonZeroU64>);

impl ModuleId {
  pub fn null() -> Self {
    Self(None)
  }

  pub fn is_null(&self) -> bool {
    self.0.is_none()
  }
}

pub struct Registry {
  pub next_module_id: u64,
  pub index: IndexMap<Ptr<String>, ModuleId>,
  pub modules: IndexMap<ModuleId, Ptr<Module>>,
}

#[derive(Debug)]
pub struct Module {
  pub name: Ptr<String>,
  pub root: Ptr<Function>,
  pub module_vars: Ptr<Table>,
}

impl Object for Module {
  fn type_name(&self) -> &'static str {
    "Module"
  }
}

impl Display for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module `{}`>", self.type_name())
  }
}

#[derive(Debug)]
pub struct ModuleDescriptor {
  pub name: Ptr<String>,
  pub root: Ptr<FunctionDescriptor>,
  pub module_vars: IndexSet<Ptr<String>>,
}

impl Object for ModuleDescriptor {
  fn type_name(&self) -> &'static str {
    "Module Descriptor"
  }
}

impl Display for ModuleDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module `{}` descriptor>", self.name)
  }
}
