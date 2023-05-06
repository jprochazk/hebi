use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;

use super::ptr::Ptr;
use super::{Object, String};
use crate::value::Value;

pub struct Table {
  data: IndexMap<Ptr<String>, Value>,
}

impl Table {
  pub fn with_capacity(n: usize) -> Self {
    Self {
      data: IndexMap::with_capacity(n),
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
    for (key, value) in self.data.iter() {
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

impl Deref for Table {
  type Target = IndexMap<Ptr<String>, Value>;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl DerefMut for Table {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.data
  }
}
