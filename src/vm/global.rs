use std::any::TypeId;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use beef::lean::Cow;
use indexmap::{IndexMap, IndexSet};

use super::DefaultModuleLoader;
use crate::object::module::{Module, ModuleId};
use crate::object::native::NativeClass;
use crate::object::{module, Ptr, Str, Table};
use crate::Result;

#[derive(Clone)]
pub struct Global {
  inner: Rc<State>,
}

pub struct State {
  globals: Ptr<Table>,
  module_registry: RefCell<module::Registry>,
  module_loader: Box<dyn module::Loader>,
  module_visited_set: RefCell<IndexSet<ModuleId>>,
  string_table: RefCell<IndexMap<Cow<'static, str>, Ptr<Str>>>,
  type_map: RefCell<IndexMap<TypeId, Ptr<NativeClass>>>,
}

impl Default for Global {
  fn default() -> Self {
    Self::new(DefaultModuleLoader {})
  }
}

impl Global {
  pub fn new(module_loader: impl module::Loader + 'static) -> Self {
    Self {
      inner: Rc::new(State {
        globals: unsafe { Ptr::alloc_raw(Table::with_capacity(0)) },
        module_registry: RefCell::new(module::Registry::new()),
        module_loader: Box::new(module_loader),
        module_visited_set: RefCell::new(IndexSet::new()),
        string_table: RefCell::new(IndexMap::new()),
        type_map: RefCell::new(IndexMap::new()),
      }),
    }
  }

  pub fn globals(&self) -> &Ptr<Table> {
    &self.globals
  }

  pub fn is_module_visited(&self, module_id: ModuleId) -> bool {
    self.module_visited_set.borrow().contains(&module_id)
  }

  pub fn get_module_by_id(&self, module_id: ModuleId) -> Option<Ptr<Module>> {
    self.module_registry.borrow().get_by_id(module_id)
  }

  pub fn get_module_by_name(&self, name: &str) -> Option<(ModuleId, Ptr<Module>)> {
    self.module_registry.borrow().get_by_name(name)
  }

  pub fn finish_module(&self, module_id: ModuleId, success: bool) {
    self.module_visited_set.borrow_mut().remove(&module_id);
    if !success {
      self.module_registry.borrow_mut().remove(module_id);
    }
  }

  pub fn next_module_id(&self) -> ModuleId {
    self.module_registry.borrow_mut().next_module_id()
  }

  pub fn load_module(&self, path: &str) -> Result<&str> {
    self.module_loader.load(path)
  }

  pub fn define_module(&self, module_id: ModuleId, name: Ptr<Str>, module: Ptr<Module>) {
    self
      .module_registry
      .borrow_mut()
      .insert(module_id, name, module);
    self.module_visited_set.borrow_mut().insert(module_id);
  }

  pub fn intern(&self, s: impl Into<Cow<'static, str>>) -> Ptr<Str> {
    let s = s.into();

    if let Some(s) = self.inner.string_table.borrow().get(&s) {
      return s.clone();
    }

    let v = self.alloc(Str::owned(s.clone()));
    self.inner.string_table.borrow_mut().insert(s, v.clone());
    v
  }

  pub fn register_type<T: Send + 'static>(&self, ty: Ptr<NativeClass>) {
    self.register_type_raw(TypeId::of::<T>(), ty);
  }

  pub fn register_type_raw(&self, type_id: TypeId, ty: Ptr<NativeClass>) {
    self.inner.type_map.borrow_mut().insert(type_id, ty);
  }

  pub fn get_type<T: Send + 'static>(&self) -> Option<Ptr<NativeClass>> {
    self
      .inner
      .type_map
      .borrow()
      .get(&TypeId::of::<T>())
      .cloned()
  }
}

impl Deref for Global {
  type Target = State;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
