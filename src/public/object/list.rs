use std::marker::PhantomData;

use super::*;
use crate::object::{list, List};
use crate::public::value::ValueRef;
use crate::{Scope, Unbind, Value};

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

  pub fn iter<'a>(&'a self) -> Iter<'a, 'cx> {
    Iter {
      inner: self.inner.iter(),
      lifetime: PhantomData,
    }
  }
}

pub struct Iter<'a, 'cx> {
  inner: list::Iter<'a>,
  lifetime: PhantomData<&'cx ()>,
}

impl<'a, 'cx> Iterator for Iter<'a, 'cx> {
  type Item = Value<'cx>;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next().map(|v| unsafe { v.bind_raw::<'cx>() })
  }
}

impl<'cx> Global<'cx> {
  pub fn new_list(&self, capacity: usize) -> ListRef<'cx> {
    self
      .inner
      .alloc(List::with_capacity(capacity))
      .bind(self.clone())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_list(&self, capacity: usize) -> ListRef<'cx> {
    self.global().new_list(capacity)
  }
}
