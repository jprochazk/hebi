use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::vec::Vec;

use super::{Object, Ptr};
use crate::util::{MAX_SAFE_INT, MIN_SAFE_INT};
use crate::value::Value;
use crate::{Result, Scope};

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

impl Object for List {
  fn keyed_field(this: super::Ptr<Self>, _: crate::Scope<'_>, key: Value) -> Result<Value> {
    let len = this.len();
    let index = to_index(key.clone(), len)?;
    let value = this
      .get(index)
      .ok_or_else(|| error!("index `{key}` out of bounds, len was `{len}`"))?;
    Ok(value)
  }

  fn keyed_field_opt(this: Ptr<Self>, _: Scope<'_>, key: Value) -> Result<Option<Value>> {
    let len = this.len();
    let index = to_index(key, len)?;
    Ok(this.get(index))
  }

  fn set_keyed_field(
    this: super::Ptr<Self>,
    _: crate::Scope<'_>,
    key: Value,
    value: Value,
  ) -> Result<()> {
    let len = this.len();
    let index = to_index(key.clone(), len)?;
    if !this.set(index, value) {
      fail!("index `{key}` out of bounds, len was `{len}`");
    };
    Ok(())
  }
}

fn to_index(index: Value, len: usize) -> Result<usize> {
  if index.is_int() {
    let index = unsafe { index.to_int().unwrap_unchecked() };
    let index = if index.is_negative() {
      len - ((-index) as usize)
    } else {
      index as usize
    };
    return Ok(index);
  } else if index.is_float() {
    let index = unsafe { index.clone().to_float().unwrap_unchecked() };
    if index.is_finite() && index.fract() == 0.0 && (MIN_SAFE_INT..=MAX_SAFE_INT).contains(&index) {
      return Ok(index as usize);
    }
  };

  fail!("`{index}` is not a valid index")
}

generate_vtable!(List);
