use std::cell::{Ref, RefCell};
use std::fmt::{Debug, Display};
use std::ops::Range;
use std::slice::SliceIndex;
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

  pub fn with_len(n: usize) -> Self {
    let mut values = Vec::new();
    values.resize_with(n, Value::none);
    Self {
      data: RefCell::new(values),
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

  pub fn push(&self, value: Value) {
    self.data.borrow_mut().push(value);
  }

  pub fn pop(&self) -> Option<Value> {
    self.data.borrow_mut().pop()
  }

  pub fn extend(&self, additional: usize) {
    let new_len = self.len() + additional;
    self.data.borrow_mut().resize_with(new_len, Value::none);
  }

  pub fn extend_from_within(&self, range: Range<usize>) {
    self.data.borrow_mut().extend_from_within(range)
  }

  pub fn extend_from_slice(&self, slice: &[Value]) {
    self.data.borrow_mut().extend_from_slice(slice)
  }

  pub fn truncate(&self, new_len: usize) {
    self.data.borrow_mut().truncate(new_len);
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

  pub fn slice<R: SliceIndex<[Value]>>(&self, range: R) -> Ref<'_, R::Output> {
    Ref::map(self.data.borrow(), |data| &data[range])
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

impl Object for List {
  fn type_name(&self) -> &'static str {
    "List"
  }
}
