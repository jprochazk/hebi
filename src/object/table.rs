use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use indexmap::{Equivalent, IndexMap};

use super::ptr::Ptr;
use super::{Object, String};
use crate::value::Value;

#[derive(Default, Clone)]
pub struct Table {
  data: RefCell<IndexMap<Ptr<String>, Value>>,
}

impl Table {
  pub fn new() -> Self {
    Self::with_capacity(0)
  }

  pub fn with_capacity(n: usize) -> Self {
    Self {
      data: RefCell::new(IndexMap::with_capacity(n)),
    }
  }

  pub fn len(&self) -> usize {
    self.data.borrow().len()
  }

  pub fn is_empty(&self) -> bool {
    self.data.borrow().is_empty()
  }

  pub fn insert(&self, key: Ptr<String>, value: Value) {
    self.data.borrow_mut().insert(key, value);
  }

  pub fn get<K: Equivalent<Ptr<String>> + ?Sized + Hash>(&self, key: &K) -> Option<Value> {
    self.data.borrow().get(key).cloned()
  }

  pub fn get_index(&self, index: usize) -> Option<Value> {
    self
      .data
      .borrow()
      .get_index(index)
      .map(|(_, value)| value.clone())
  }

  pub fn set_index(&self, index: usize, value: Value) -> bool {
    // TODO: handle error
    match self.data.borrow_mut().get_index_mut(index) {
      Some((_, slot)) => {
        *slot = value;
        true
      }
      None => false,
    }
  }
}

impl Display for Table {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<table>")
  }
}

impl Debug for Table {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut s = f.debug_struct("Table");
    for (key, value) in self.data.borrow().iter() {
      s.field(key, value);
    }
    s.finish()
  }
}

impl Object for Table {
  fn type_name(&self) -> &'static str {
    "Table"
  }
}
