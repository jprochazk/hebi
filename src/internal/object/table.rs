use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use indexmap::{Equivalent, IndexMap};

use super::ptr::Ptr;
use super::{Object, Str};
use crate::internal::error::Result;
use crate::internal::value::Value;
use crate::public::Scope;

#[derive(Default)]
pub struct Table {
  data: RefCell<IndexMap<Ptr<Str>, Value>>,
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

  #[allow(dead_code)] // TODO: expose
  pub fn is_empty(&self) -> bool {
    self.data.borrow().is_empty()
  }

  pub fn insert(&self, key: Ptr<Str>, value: Value) -> Option<Value> {
    self.data.borrow_mut().insert(key, value)
  }

  pub fn get<K: Equivalent<Ptr<Str>> + ?Sized + Hash>(&self, key: &K) -> Option<Value> {
    self.data.borrow().get(key).cloned()
  }

  pub fn set<K: Equivalent<Ptr<Str>> + ?Sized + Hash>(&self, key: &K, value: Value) -> bool {
    if let Some(slot) = self.data.borrow_mut().get_mut(key) {
      *slot = value;
      true
    } else {
      false
    }
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

  pub fn keys(&self) -> Keys {
    Keys {
      table: self,
      index: 0,
    }
  }

  pub fn values(&self) -> Values {
    Values {
      table: self,
      index: 0,
    }
  }

  pub fn entries(&self) -> Entries {
    Entries {
      table: self,
      index: 0,
    }
  }

  pub fn copy(&self) -> Self {
    Self {
      data: self.data.clone(),
    }
  }
}

pub struct Keys<'a> {
  table: &'a Table,
  index: usize,
}

impl<'a> Iterator for Keys<'a> {
  type Item = Ptr<Str>;

  fn next(&mut self) -> Option<Self::Item> {
    match self.table.data.borrow().get_index(self.index) {
      Some((key, _)) => {
        self.index += 1;
        Some(key.clone())
      }
      None => None,
    }
  }
}

pub struct Values<'a> {
  table: &'a Table,
  index: usize,
}

impl<'a> Iterator for Values<'a> {
  type Item = Value;

  fn next(&mut self) -> Option<Self::Item> {
    match self.table.data.borrow().get_index(self.index) {
      Some((_, value)) => {
        self.index += 1;
        Some(value.clone())
      }
      None => None,
    }
  }
}

pub struct Entries<'a> {
  table: &'a Table,
  index: usize,
}

impl<'a> Iterator for Entries<'a> {
  type Item = (Ptr<Str>, Value);

  fn next(&mut self) -> Option<Self::Item> {
    match self.table.data.borrow().get_index(self.index) {
      Some((key, value)) => {
        self.index += 1;
        Some((key.clone(), value.clone()))
      }
      None => None,
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
    let mut s = f.debug_map();
    for (key, value) in self.data.borrow().iter() {
      s.entry(key, value);
    }
    s.finish()
  }
}

impl Object for Table {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Table"
  }

  default_instance_of!();

  fn keyed_field(_: Scope<'_>, this: Ptr<Self>, key: Value) -> Result<Value> {
    let Some(key) = key.clone().to_object::<Str>() else {
      fail!("`{key}` is not a string");
    };
    let value = this
      .get(key.as_str())
      .ok_or_else(|| error!("`{this}` has no index `{key}`"))?;
    Ok(value)
  }

  fn keyed_field_opt(_: Scope<'_>, this: Ptr<Self>, key: Value) -> Result<Option<Value>> {
    let Some(key) = key.clone().to_object::<Str>() else {
      fail!("`{key}` is not a string");
    };
    let value = this.get(key.as_str());
    Ok(value)
  }

  fn set_keyed_field(_: Scope<'_>, this: Ptr<Self>, key: Value, value: Value) -> Result<()> {
    let Some(key) = key.clone().to_object::<Str>() else {
      fail!("`{key}` is not a string");
    };
    this.insert(key, value);
    Ok(())
  }

  fn eq(scope: Scope<'_>, this: Ptr<Self>, other: Ptr<Self>) -> Result<bool> {
    if this.len() != other.len() {
      return Ok(false);
    }

    let this_data = this.data.borrow();
    let other_data = other.data.borrow();
    for (this_key, this_value) in this_data.iter() {
      let other_value = match other_data.get(this_key) {
        Some(value) => value,
        None => return Ok(false),
      };

      let are_equal = scope.are_equal(this_value.clone(), other_value.clone())?;
      if !are_equal {
        return Ok(false);
      }
    }

    Ok(true)
  }
}

declare_object_type!(Table);
