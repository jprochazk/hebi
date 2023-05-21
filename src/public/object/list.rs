use std::marker::PhantomData;

use super::*;
use crate::object::{list, List as OwnedList, Ptr};
use crate::{Hebi, Scope, Unbind, Value};

decl_ref! {
  struct List(Ptr<OwnedList>)
}

impl_object_ref!(List, OwnedList);

impl<'cx> List<'cx> {
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  pub fn push(&self, value: Value<'cx>) {
    self.inner.push(value.unbind());
  }

  pub fn pop(&self) -> Option<Value<'cx>> {
    self
      .inner
      .pop()
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn get(&self, index: usize) -> Option<Value<'cx>> {
    self
      .inner
      .get(index)
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  #[must_use = "`set` returns false if index is out of bounds"]
  pub fn set(&self, index: usize, value: Value<'cx>) -> bool {
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
  pub fn new_list(&self, capacity: usize) -> List<'cx> {
    self
      .inner
      .alloc(OwnedList::with_capacity(capacity))
      .bind(self.clone())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_list(&self, capacity: usize) -> List<'cx> {
    self.global().new_list(capacity)
  }
}

impl Hebi {
  pub fn new_list(&self, capacity: usize) -> List {
    self.global().new_list(capacity)
  }
}
