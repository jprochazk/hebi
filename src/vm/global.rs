use std::any::TypeId;
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use beef::lean::Cow;
use indexmap::{IndexMap, IndexSet};

use super::Config;
use crate::error::Result;
use crate::object::module::{Module, ModuleId};
use crate::object::native::NativeClass;
use crate::object::{module, table, Ptr, Str, Table};
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Global {
  inner: Rc<State>,
}

pub trait IoBase: Send + Sync + 'static {
  fn as_any(&self) -> &dyn std::any::Any;
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: Send + Sync + 'static> IoBase for T {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

pub trait Output: std::io::Write + IoBase {}
impl<T: std::io::Write + IoBase> Output for T {}

pub trait Input: std::io::Read + IoBase {}
impl<T: std::io::Read + IoBase> Input for T {}

pub struct State {
  globals: Ptr<Table>,
  io: Io,
  module_registry: RefCell<module::Registry>,
  module_loader: Box<dyn module::ModuleLoader>,
  module_visited_set: RefCell<IndexSet<ModuleId>>,
  string_table: RefCell<IndexMap<Cow<'static, str>, Ptr<Str>>>,
  type_map: RefCell<IndexMap<TypeId, Ptr<NativeClass>>>,
}

impl Debug for State {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("State")
      .field("globals", &self.globals)
      .field("io", &"<...>")
      .field("module_registry", &self.module_registry)
      .field("module_loader", &"<...>")
      .field("module_visited_set", &self.module_visited_set)
      .field("string_table", &self.string_table)
      .field("type_map", &self.type_map)
      .finish()
  }
}

pub struct Io {
  pub(crate) input: RefCell<Box<dyn Input>>,
  pub(crate) output: RefCell<Box<dyn Output>>,
}

impl Io {
  pub fn new(input: Box<dyn Input>, output: Box<dyn Output>) -> Self {
    Self {
      input: RefCell::new(Box::new(input)),
      output: RefCell::new(Box::new(output)),
    }
  }
}

impl Default for Io {
  fn default() -> Self {
    Self::new(Box::new(std::io::stdin()), Box::new(std::io::stdout()))
  }
}

impl Default for Global {
  fn default() -> Self {
    Self::new(Config::default())
  }
}

impl Global {
  pub fn new(config: Config) -> Self {
    let (module_loader, input, output) = config.resolve();
    let io = Io {
      input: RefCell::new(input),
      output: RefCell::new(output),
    };

    Self {
      inner: Rc::new(State {
        globals: unsafe { Ptr::alloc_raw(Table::with_capacity(0)) },
        io,
        module_registry: RefCell::new(module::Registry::new()),
        module_loader,
        module_visited_set: RefCell::new(IndexSet::new()),
        string_table: RefCell::new(IndexMap::new()),
        type_map: RefCell::new(IndexMap::new()),
      }),
    }
  }

  pub fn get(&self, key: &str) -> Option<Value> {
    self.globals.get(key)
  }

  pub fn set(&self, key: Ptr<Str>, value: Value) {
    self.globals.insert(key, value);
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

  pub fn load_module(&self, path: &str) -> Result<Cow<'static, str>> {
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

  pub fn io(&self) -> &Io {
    &self.inner.io
  }

  pub fn entries(&self) -> table::Entries<'_> {
    self.inner.globals.entries()
  }
}

impl Deref for Global {
  type Target = State;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
