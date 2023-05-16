use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use indexmap::{Equivalent, IndexMap};

use super::ptr::Ptr;
use super::{Object, String};
use crate as hebi;
use crate::value::Value;

#[derive(Default)]
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

  pub fn set<K: Equivalent<Ptr<String>> + ?Sized + Hash>(&self, key: &K, value: Value) -> bool {
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

  pub fn keys(&self) -> Keys<'_> {
    Keys {
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
  type Item = Ptr<String>;

  fn next(&mut self) -> Option<Self::Item> {
    match self
      .table
      .data
      .borrow()
      .get_index(self.index)
      .map(|(key, _)| key.clone())
    {
      Some(key) => {
        self.index += 1;
        Some(key)
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
  fn type_name(&self) -> &'static str {
    "Table"
  }

  fn named_field(
    &self,
    cx: &crate::ctx::Context,
    name: Ptr<String>,
  ) -> crate::Result<Option<Value>> {
    let _ = cx;
    Ok(self.get(&name))
  }

  fn set_named_field(
    &self,
    cx: &crate::ctx::Context,
    name: Ptr<String>,
    value: Value,
  ) -> crate::Result<()> {
    let _ = cx;
    self.insert(name, value);
    Ok(())
  }

  fn keyed_field(&self, cx: &crate::ctx::Context, key: Value) -> crate::Result<Option<Value>> {
    let _ = cx;
    let Some(key) = key.clone().to_object::<String>() else {
      hebi::fail!("`{key}` is not a string");
    };
    Ok(self.get(&key))
  }

  fn set_keyed_field(&self, cx: &hebi::ctx::Context, key: Value, value: Value) -> hebi::Result<()> {
    let _ = cx;
    let Some(key) = key.clone().to_object::<String>() else {
      hebi::fail!("`{key}` is not a string");
    };
    self.insert(key, value);
    Ok(())
  }
}
