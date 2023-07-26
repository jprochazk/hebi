use core::cell::{Cell, RefCell};
use core::fmt::{Debug, Display};

use super::func::{Function, FunctionProto};
use super::map::Map;
use super::string::Str;
use crate::ds::map::{GcHashMapN, GcOrdHashMapN};
use crate::ds::{fx, HasNoAlloc};
use crate::error::{AllocError, Result};
use crate::gc::{Gc, Object, Ref, NO_ALLOC};
use crate::val::{nil, Value};

pub trait ModuleLoader: Send {
  fn load(&self, path: &str) -> Result<&str>;
}

#[derive(Debug)]
pub struct ModuleRegistry {
  next_id: Cell<u64>,
  index: RefCell<GcHashMapN<Ref<Str>, ModuleId>>,
  modules: RefCell<GcHashMapN<ModuleId, Ref<Module>>>,
}

impl ModuleRegistry {
  pub fn new(gc: &Gc) -> Result<Ref<Self>, AllocError> {
    gc.try_alloc(Self {
      next_id: Cell::new(0),
      index: RefCell::new(GcHashMapN::with_hasher_in(fx(), NO_ALLOC)),
      modules: RefCell::new(GcHashMapN::with_hasher_in(fx(), NO_ALLOC)),
    })
  }

  pub fn next_id(&self) -> ModuleId {
    let id = self.next_id.get();
    self.next_id.set(id + 1);
    ModuleId(id)
  }

  pub fn try_insert(&self, gc: &Gc, module: Ref<Module>) -> Result<(), AllocError> {
    let id = module.id();
    let name = module.name();
    let mut index = self.index.borrow_mut();
    let mut modules = self.modules.borrow_mut();
    let index = index.as_alloc_mut(gc);
    let modules = modules.as_alloc_mut(gc);
    index.try_reserve(1)?;
    modules.try_reserve(1)?;
    index.insert(name, id);
    modules.insert(id, module);

    Ok(())
  }

  pub fn remove(&self, id: ModuleId) -> Option<Ref<Module>> {
    let mut index = self.index.borrow_mut();
    let mut modules = self.modules.borrow_mut();
    let module = modules.remove(&id)?;
    index.remove(&module.name());
    Some(module)
  }

  pub fn by_id(&self, id: ModuleId) -> Option<Ref<Module>> {
    self.modules.borrow().get(&id).copied()
  }

  pub fn by_name(&self, name: &str) -> Option<(ModuleId, Ref<Module>)> {
    let id = self.index.borrow().get(name).copied()?;
    let module = self.modules.borrow().get(&id).copied()?;
    Some((id, module))
  }
}

impl Display for ModuleRegistry {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("<module registry>")
  }
}

impl Object for ModuleRegistry {
  const NEEDS_DROP: bool = false;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u64);

impl Display for ModuleId {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[derive(Debug)]
pub struct Module {
  proto: Ref<ModuleProto>,
  root: Option<Ref<Function>>,
  vars: Ref<Map>,
}

impl Module {
  pub fn new(
    gc: &Gc,
    proto: Ref<ModuleProto>,
    root: Option<Ref<Function>>,
  ) -> Result<Ref<Self>, AllocError> {
    let vars = Map::try_with_capacity_in(gc, proto.vars.len())?;
    for (k, _) in proto.vars.iter() {
      let _ = vars.try_insert_no_grow(*k, Value::new(nil));
    }
    gc.try_alloc(Self { proto, root, vars })
  }

  pub fn id(&self) -> ModuleId {
    self.proto.id
  }

  pub fn name(&self) -> Ref<Str> {
    self.proto.name
  }

  pub fn root(&self) -> Option<Ref<Function>> {
    self.root
  }

  pub fn vars(&self) -> Ref<Map> {
    self.vars
  }
}

impl Display for Module {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<module `{}`>", self.name())
  }
}

impl Object for Module {}

#[derive(Debug)]
pub struct ModuleProto {
  id: ModuleId,
  name: Ref<Str>,
  root: Ref<FunctionProto>,
  // TODO: GcOrdHashSet
  vars: GcOrdHashMapN<Ref<Str>, ()>,
}

impl ModuleProto {
  pub fn new(
    gc: &Gc,
    id: ModuleId,
    name: &str,
    root: Ref<FunctionProto>,
    vars: GcOrdHashMapN<Ref<Str>, ()>,
  ) -> Result<Ref<Self>, AllocError> {
    let name = Str::new(gc, name)?;
    gc.try_alloc(ModuleProto {
      id,
      name,
      root,
      vars,
    })
  }

  pub fn name(&self) -> Ref<Str> {
    self.name
  }

  pub fn root(&self) -> Ref<FunctionProto> {
    self.root
  }

  pub fn vars(&self) -> &GcOrdHashMapN<Ref<Str>, ()> {
    &self.vars
  }
}

impl Display for ModuleProto {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<module `{}`>", self.name)
  }
}

impl Object for ModuleProto {
  const NEEDS_DROP: bool = false;
}
