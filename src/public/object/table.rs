use super::*;
use crate::object::Table;
use crate::Scope;

decl_object_ref! {
  struct Table
}

impl<'cx> TableRef<'cx> {}

impl<'cx> Global<'cx> {
  pub fn new_table(&self, capacity: usize) -> TableRef<'cx> {
    self
      .inner
      .alloc(Table::with_capacity(capacity))
      .bind(self.clone())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_table(&self, capacity: usize) -> TableRef<'cx> {
    self.global().new_table(capacity)
  }
}
