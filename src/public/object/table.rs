use std::marker::PhantomData;

use super::*;
use crate::internal::object::{table, Ptr, Table as OwnedTable};
use crate::public::{Hebi, Scope, Str, Unbind, Value};

decl_ref! {
  struct Table(Ptr<OwnedTable>)
}

impl_object_ref!(Table, OwnedTable);

impl<'cx> Table<'cx> {
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  pub fn insert(&self, key: Str<'cx>, value: Value<'cx>) -> Option<Value<'cx>> {
    self
      .inner
      .insert(key.unbind(), value.unbind())
      .map(|v| unsafe { v.bind_raw::<'cx>() })
  }

  pub fn get(&self, key: &str) -> Option<Value<'cx>> {
    self.inner.get(key).map(|v| unsafe { v.bind_raw::<'cx>() })
  }

  pub fn keys<'a>(&'a self) -> Keys<'a, 'cx> {
    Keys {
      inner: self.inner.keys(),
      lifetime: PhantomData,
    }
  }

  pub fn values<'a>(&'a self) -> Values<'a, 'cx> {
    Values {
      inner: self.inner.values(),
      lifetime: PhantomData,
    }
  }

  pub fn entries<'a>(&'a self) -> Entries<'a, 'cx> {
    Entries {
      inner: self.inner.entries(),
      lifetime: PhantomData,
    }
  }
}

pub struct Keys<'a, 'cx> {
  inner: table::Keys<'a>,
  lifetime: PhantomData<&'cx ()>,
}

impl<'a, 'cx> Iterator for Keys<'a, 'cx> {
  type Item = Str<'cx>;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next().map(|v| unsafe { v.bind_raw::<'cx>() })
  }
}

pub struct Values<'a, 'cx> {
  inner: table::Values<'a>,
  lifetime: PhantomData<&'cx ()>,
}

impl<'a, 'cx> Iterator for Values<'a, 'cx> {
  type Item = Value<'cx>;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.next().map(|v| unsafe { v.bind_raw::<'cx>() })
  }
}

pub struct Entries<'a, 'cx> {
  inner: table::Entries<'a>,
  lifetime: PhantomData<&'cx ()>,
}

impl<'a, 'cx> Iterator for Entries<'a, 'cx> {
  type Item = (Str<'cx>, Value<'cx>);

  fn next(&mut self) -> Option<Self::Item> {
    self
      .inner
      .next()
      .map(|(key, value)| unsafe { (key.bind_raw::<'cx>(), value.bind_raw::<'cx>()) })
  }
}

impl<'cx> Global<'cx> {
  pub fn new_table(&self, capacity: usize) -> Table<'cx> {
    self
      .inner
      .alloc(OwnedTable::with_capacity(capacity))
      .bind(self.clone())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_table(&self, capacity: usize) -> Table<'cx> {
    self.global().new_table(capacity)
  }
}

impl Hebi {
  pub fn new_table(&self, capacity: usize) -> Table {
    self.global().new_table(capacity)
  }
}
