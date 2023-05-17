use std::cell::{Ref, RefCell, RefMut};
use std::ops::Deref;
use std::rc::Rc;

use indexmap::IndexSet;

use crate::ctx::Context;
use crate::object::module::ModuleId;
use crate::object::{module, Ptr, Table};

#[derive(Clone)]
pub struct Global(Rc<State>);

pub struct State {
  globals: Ptr<Table>,
  module_registry: RefCell<module::Registry>,
  module_loader: RefCell<Box<dyn module::Loader>>,
  module_visited_set: RefCell<IndexSet<ModuleId>>,
  // types: Types,
  // modules: Modules,
}

impl Global {
  pub fn new(cx: &Context, module_loader: impl module::Loader + 'static) -> Self {
    Self(Rc::new(State {
      globals: cx.alloc(Table::with_capacity(0)),
      module_registry: RefCell::new(module::Registry::new()),
      module_loader: RefCell::new(Box::new(module_loader)),
      module_visited_set: RefCell::new(IndexSet::new()),
    }))
  }

  pub fn globals(&self) -> &Ptr<Table> {
    &self.globals
  }

  pub(super) fn module_registry(&self) -> Ref<'_, module::Registry> {
    self.module_registry.borrow()
  }

  pub(super) fn module_registry_mut(&mut self) -> RefMut<'_, module::Registry> {
    self.module_registry.borrow_mut()
  }

  #[allow(dead_code)] // used in tests
  pub(super) fn set_module_loader(&self, module_loader: impl module::Loader + 'static) {
    self.module_loader.replace(Box::new(module_loader));
  }

  pub(super) fn module_loader(&self) -> Ref<'_, dyn module::Loader> {
    Ref::map(self.module_loader.borrow(), |v| v.deref())
  }

  pub(super) fn module_visited_set(&self) -> Ref<'_, IndexSet<ModuleId>> {
    self.module_visited_set.borrow()
  }

  pub(super) fn module_visited_set_mut(&self) -> RefMut<'_, IndexSet<ModuleId>> {
    self.module_visited_set.borrow_mut()
  }
}

impl Deref for Global {
  type Target = State;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
