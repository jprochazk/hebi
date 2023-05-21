use std::fmt::{Debug, Display};
use std::num::NonZeroU64;

use indexmap::{IndexMap, IndexSet};

use super::native::{NativeAsyncFunction, NativeClass, NativeFunction};
use super::ptr::Ptr;
use super::{Function, FunctionDescriptor, Object, String, Table};
use crate as hebi;
use crate::module::NativeModule;
use crate::value::Value;
use crate::vm::global::Global;
use crate::Scope;

pub trait Loader {
  fn load(&self, path: &str) -> hebi::Result<&str>;
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
    let module = self.modules.remove(&id)?;
    self.index.remove(module.name.as_str());
    Some(module)
  }

  pub fn get_by_id(&self, id: ModuleId) -> Option<Ptr<Module>> {
    self.modules.get(&id).cloned()
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
  pub module_vars: Ptr<Table>,
  pub kind: ModuleKind,
}

#[derive(Debug)]
pub enum ModuleKind {
  Script { root: Ptr<Function> },
  Native,
}

impl Module {
  pub fn script(
    global: Global,
    name: Ptr<String>,
    root: Ptr<Function>,
    module_vars: &IndexSet<Ptr<String>>,
    module_id: ModuleId,
  ) -> Self {
    let module_vars = {
      let table = global.alloc(Table::with_capacity(module_vars.len()));
      for var in module_vars {
        table.insert(var.clone(), Value::none());
      }
      table
    };

    Self {
      module_id,
      name,
      module_vars,
      kind: ModuleKind::Script { root },
    }
  }

  pub fn native(
    global: Global,
    name: Ptr<String>,
    module: &NativeModule,
    module_id: ModuleId,
  ) -> Self {
    let module_vars = global.alloc(Table::with_capacity(module.data.fns.len()));

    for (name, f) in module.data.fns.iter() {
      let name = global.alloc(String::owned(name.clone()));
      let f = Value::object(global.alloc(NativeFunction {
        name: name.clone(),
        cb: f.clone(),
      }));
      module_vars.insert(name, f);
    }

    for (name, f) in module.data.async_fns.iter() {
      let name = global.alloc(String::owned(name.clone()));
      let f = Value::object(global.alloc(NativeAsyncFunction {
        name: name.clone(),
        cb: f.clone(),
      }));
      module_vars.insert(name, f);
    }

    for (name, desc) in module.data.classes.iter() {
      let name = global.alloc(String::owned(name.clone()));
      let class = global.alloc(NativeClass::new(global.clone(), desc));
      global.register_type_raw(class.type_id, class.clone());
      module_vars.insert(name, Value::object(class));
    }

    Self {
      module_id,
      name,
      module_vars,
      kind: ModuleKind::Native,
    }
  }
}

impl Object for Module {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Module"
  }

  fn named_field(this: Ptr<Self>, _: Scope<'_>, name: Ptr<String>) -> hebi::Result<Option<Value>> {
    Ok(this.module_vars.get(&name))
  }
}

generate_vtable!(Module);

impl Display for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module `{}`>", self.name)
  }
}

#[derive(Debug)]
pub struct ModuleDescriptor {
  pub name: Ptr<String>,
  pub root: Ptr<FunctionDescriptor>,
  pub module_vars: IndexSet<Ptr<String>>,
}

impl Object for ModuleDescriptor {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Module Descriptor"
  }
}

generate_vtable!(ModuleDescriptor);

impl Display for ModuleDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module `{}` descriptor>", self.name)
  }
}
