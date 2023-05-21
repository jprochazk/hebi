use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::vec::Vec;

use super::Object;
use crate::value::Value;

#[derive(Default)]
pub struct List {
  data: RefCell<Vec<Value>>,
}

impl List {
  pub fn new() -> Self {
    Self::with_capacity(0)
  }

  pub fn with_capacity(n: usize) -> Self {
    Self {
      data: RefCell::new(Vec::with_capacity(n)),
    }
  }

  pub fn len(&self) -> usize {
    self.data.borrow().len()
  }

  pub fn is_empty(&self) -> bool {
    self.data.borrow().is_empty()
  }

  #[allow(dead_code)] // TODO: use in `index` impl
  pub fn get(&self, index: usize) -> Option<Value> {
    self.data.borrow().get(index).cloned()
  }

  pub fn push(&self, value: Value) {
    self.data.borrow_mut().push(value);
  }

  pub fn pop(&self) -> Option<Value> {
    self.data.borrow_mut().pop()
  }

  /// # Safety
  ///
  /// - `index` must be within the bounds of `self`
  pub unsafe fn get_unchecked(&self, index: usize) -> Value {
    debug_assert!(index < self.len(), "index {index} out of bounds");
    self.data.borrow().get_unchecked(index).clone()
  }

  #[must_use = "`set` returns false if index is out of bounds"]
  pub fn set(&self, index: usize, value: Value) -> bool {
    if let Some(slot) = self.data.borrow_mut().get_mut(index) {
      *slot = value;
      true
    } else {
      false
    }
  }

  /// # Safety
  ///
  /// - `index` must be within the bounds of `self`
  pub unsafe fn set_unchecked(&self, index: usize, value: Value) {
    debug_assert!(index < self.len(), "index {index} out of bounds");
    *self.data.borrow_mut().get_mut(index).unwrap_unchecked() = value;
  }

  pub fn iter(&self) -> Iter {
    Iter {
      list: self,
      index: 0,
    }
  }
}

pub struct Iter<'a> {
  list: &'a List,
  index: usize,
}

impl<'a> Iterator for Iter<'a> {
  type Item = Value;

  fn next(&mut self) -> Option<Self::Item> {
    match self.list.data.borrow().get(self.index) {
      Some(value) => {
        self.index += 1;
        Some(value.clone())
      }
      None => None,
    }
  }
}

impl From<Vec<Value>> for List {
  fn from(values: Vec<Value>) -> Self {
    Self {
      data: RefCell::new(values),
    }
  }
}

impl Display for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<list>")
  }
}

impl Debug for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_list().entries(&self.data.borrow()[..]).finish()
  }
}

impl Object for List {}

generate_vtable!(List);
