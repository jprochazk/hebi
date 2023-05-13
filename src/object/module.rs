use std::fmt::{Debug, Display};
use std::num::NonZeroU64;

use indexmap::{IndexMap, IndexSet};

use super::ptr::Ptr;
use super::{Function, FunctionDescriptor, Object, String, Table};
use crate::ctx::Context;
use crate::value::Value;

pub trait Loader {
  fn load(&self, path: &str) -> Option<&str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(Option<NonZeroU64>);

impl ModuleId {
  /// Identifies the global module.
  ///
  /// A function executing in the context of a global module
  /// stores its module variables in the global object.
  ///
  /// An example usage of this is in `Hebi::eval`.
  pub fn global() -> Self {
    Self(None)
  }

  pub fn is_global(&self) -> bool {
    self.0.is_none()
  }
}

impl Display for ModuleId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.0 {
      Some(id) => write!(f, "{id}"),
      None => write!(f, "global"),
    }
  }
}

pub struct Registry {
  pub next_module_id: NonZeroU64,
  pub index: IndexMap<Ptr<String>, ModuleId>,
  pub modules: IndexMap<ModuleId, Ptr<Module>>,
}

impl Registry {
  pub fn new() -> Self {
    Self {
      next_module_id: unsafe { NonZeroU64::new_unchecked(1) },
      index: IndexMap::new(),
      modules: IndexMap::new(),
    }
  }

  pub fn next_module_id(&mut self) -> ModuleId {
    let temp = ModuleId(Some(self.next_module_id));
    self.next_module_id = self.next_module_id.saturating_add(1);
    temp
  }

  pub fn insert(&mut self, id: ModuleId, name: Ptr<String>, module: Ptr<Module>) {
    self.index.insert(name, id);
    self.modules.insert(id, module);
  }

  pub fn remove(&mut self, id: ModuleId) -> Option<Ptr<Module>> {
    // TODO: remove from index
    self.modules.remove(&id)
  }

  pub fn get_by_id(&self, id: ModuleId) -> Option<Ptr<Module>> {
    self.modules.get(&id).cloned()
  }

  pub fn contains_by_name(&self, name: &str) -> bool {
    self.index.contains_key(name)
  }

  pub fn get_by_name(&self, name: &str) -> Option<(ModuleId, Ptr<Module>)> {
    let module_id = self.index.get(name).copied()?;
    let module = self.modules.get(&module_id).cloned()?;
    Some((module_id, module))
  }
}

impl Default for Registry {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Debug)]
pub struct Module {
  pub module_id: ModuleId,
  pub name: Ptr<String>,
  pub root: Ptr<Function>,
  pub module_vars: Ptr<Table>,
}

impl Module {
  pub fn new(
    cx: &Context,
    name: Ptr<String>,
    root: Ptr<Function>,
    module_vars: &IndexSet<Ptr<String>>,
    module_id: ModuleId,
  ) -> Self {
    let module_vars = {
      let table = cx.alloc(Table::with_capacity(module_vars.len()));
      for var in module_vars {
        table.insert(var.clone(), Value::none());
      }
      table
    };

    Self {
      module_id,
      name,
      root,
      module_vars,
    }
  }
}

impl Object for Module {
  fn type_name(&self) -> &'static str {
    "Module"
  }

  fn named_field(&self, _: &Context, name: &str) -> crate::error::Result<Option<Value>> {
    Ok(self.module_vars.get(name))
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
