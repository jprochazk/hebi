use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use runtime::value::object::handle::Handle;
use runtime::value::object::Str;

#[derive(Clone)]
pub struct Context(Arc<Mutex<Inner>>);

struct Inner {
  intern_table: IndexMap<String, Handle<Str>>,
}

impl Context {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self(Arc::new(Mutex::new(Inner {
      intern_table: IndexMap::new(),
    })))
  }

  pub fn alloc_string(&self, str: &str) -> Handle<Str> {
    let mut inner = self.0.lock().unwrap();
    if let Some(str) = inner.intern_table.get(str) {
      str.clone()
    } else {
      let value = Handle::new(str);
      inner.intern_table.insert(str.into(), value.clone());
      value
    }
  }
}
