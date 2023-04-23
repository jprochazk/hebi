use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use std::vec::Vec;

use crate::value::Value;

pub struct List {
  data: Vec<Value>,
}

impl Display for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<list>")
  }
}

impl Debug for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_list().entries(&self.data[..]).finish()
  }
}

impl Deref for List {
  type Target = Vec<Value>;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl DerefMut for List {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.data
  }
}
