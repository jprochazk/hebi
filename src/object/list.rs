use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::vec::Vec;

use super::Object;
use crate::value::Value;

pub struct List {
  data: RefCell<Vec<Value>>,
}

impl List {
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

  pub fn get(&self, index: usize) -> Option<Value> {
    self.data.borrow().get(index).cloned()
  }

  /// # Safety
  ///
  /// - `index` must be within the bounds of `self`
  pub unsafe fn get_unchecked(&self, index: usize) -> Value {
    debug_assert!(index < self.len(), "index {index} out of bounds");
    self.data.borrow().get_unchecked(index).clone()
  }

  pub fn set(&self, index: usize, value: Value) {
    if let Some(slot) = self.data.borrow_mut().get_mut(index) {
      *slot = value;
    }
  }

  /// # Safety
  ///
  /// - `index` must be within the bounds of `self`
  pub unsafe fn set_unchecked(&self, index: usize, value: Value) {
    debug_assert!(index < self.len(), "index {index} out of bounds");
    *self.data.borrow_mut().get_mut(index).unwrap_unchecked() = value;
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

impl Object for List {
  fn type_name(&self) -> &'static str {
    "List"
  }
}
