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
