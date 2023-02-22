use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexSet;

use crate::value::handle::Handle;
use crate::value::object::{ObjectType, Str};

#[derive(Clone)]
pub struct Context {
  inner: Rc<RefCell<Inner>>,
}

pub struct Inner {
  intern_table: IndexSet<Handle<Str>>,
}

impl Context {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self {
      inner: Rc::new(RefCell::new(Inner {
        intern_table: IndexSet::new(),
      })),
    }
  }

  pub fn alloc_interned_string(&self, str: impl AsRef<str>) -> Handle<Str> {
    let str = str.as_ref();
    {
      if let Some(str) = self.inner.borrow().intern_table.get(str) {
        return str.clone();
      }
    }
    {
      let value = self.alloc(Str::from(str));
      self.inner.borrow_mut().intern_table.insert(value.clone());
      value
    }
  }

  pub fn alloc<T: ObjectType>(&self, value: T) -> Handle<T> {
    Handle::_alloc(value)
  }
}
