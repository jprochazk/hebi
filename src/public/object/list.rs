use super::*;
use crate::object::List;
use crate::public::value::ValueRef;
use crate::Unbind;

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
}
