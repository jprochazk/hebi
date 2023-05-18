use super::*;
use crate::object::List;
use crate::public::value::ValueRef;
use crate::{Scope, Unbind};

decl_object_ref! {
  struct List
}

impl<'cx> ListRef<'cx> {
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  pub fn push(&self, value: ValueRef<'cx>) {
    self.inner.push(value.unbind());
  }

  pub fn pop(&self) -> Option<ValueRef<'cx>> {
    self
      .inner
      .pop()
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn get(&self, index: usize) -> Option<ValueRef<'cx>> {
    self
      .inner
      .get(index)
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  #[must_use = "`set` returns false if index is out of bounds"]
  pub fn set(&self, index: usize, value: ValueRef<'cx>) -> bool {
    self.inner.set(index, value.unbind())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_list(&self, capacity: usize) -> ListRef<'cx> {
    self
      .cx()
      .inner
      .alloc(List::with_capacity(capacity))
      .bind(self.cx())
  }
}
