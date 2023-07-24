//! Hebi's Table type.
//!
//! [`Table`] is implemented as an ordered hash map.
//! It is _not_ resistant to HashDOS by default.
//!
//! Implementation is heavily inspired by [indexmap](https://docs.rs/indexmap/latest/indexmap/).

use core::cell::UnsafeCell;
use core::fmt::{Debug, Display};
use core::hash::BuildHasherDefault;

use hashbrown::HashSet;
use rustc_hash::FxHasher;

use super::string::Str;
use crate::ds::map::{GcOrdHashMap, GcOrdHashMapN};
use crate::ds::set::GcHashSet;
use crate::ds::{fx, HasAlloc, HasNoAlloc};
use crate::error::AllocError;
use crate::gc::{Alloc, Gc, NoAlloc, Object, Ref, NO_ALLOC};
use crate::op::Reg;
use crate::util::DelegateDebugToDisplay;
use crate::val::Value;

pub struct Table {
  map: UnsafeCell<GcOrdHashMapN<Ref<Str>, Value>>,
}

impl Table {
  /// Allocates a new table.
  ///
  /// The table is initially empty.
  /// The only allocation done here is to put the `Table` object
  /// onto the garbage-collected heap.
  pub fn try_new_in(gc: &Gc) -> Result<Ref<Self>, AllocError> {
    let map = UnsafeCell::new(GcOrdHashMapN::new_in(NO_ALLOC));
    gc.try_alloc(Table { map })
  }

  /// Allocates a new table with space for at least `capacity` entries.
  pub fn try_with_capacity_in(gc: &Gc, capacity: usize) -> Result<Ref<Self>, AllocError> {
    let map = GcOrdHashMap::try_with_capacity_in(capacity, Alloc::new(gc))?;
    let map = UnsafeCell::new(map.to_no_alloc());
    gc.try_alloc(Table { map })
  }

  /// Returns the number of entries currently in the table.
  #[inline]
  pub fn len(&self) -> usize {
    self.map().len()
  }

  /// Returns the table's remaining capacity.
  #[inline]
  pub fn capacity(&self) -> usize {
    self.map().capacity()
  }

  /// Returns `true` if the table is empty.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.map().is_empty()
  }

  /// Inserts `value` into the table associated with the key `key`.
  pub fn try_insert(
    &self,
    gc: &Gc,
    key: Ref<Str>,
    value: Value,
  ) -> Result<Option<Value>, AllocError> {
    let map = self.map_alloc(gc);
    map.try_reserve(1)?;
    Ok(unsafe { map.try_insert_no_grow(key, value).unwrap_unchecked() })
  }

  /// Removes `key` from the table, returning it if it exists.
  pub fn remove(&self, key: &str) -> Option<Value> {
    self.map_mut().remove(key)
  }

  /// Like [`Table::try_insert`], but will return the `(key, value)` pair
  /// if the table does not have enough spare capacity.
  pub fn try_insert_no_grow(
    &self,
    key: Ref<Str>,
    value: Value,
  ) -> Result<Option<Value>, (Ref<Str>, Value)> {
    self.map_mut().try_insert_no_grow(key, value)
  }

  #[inline]
  pub fn try_reserve(&self, gc: &Gc, additional: usize) -> Result<(), AllocError> {
    self.map_alloc(gc).try_reserve(additional)
  }

  pub fn get(&self, key: &str) -> Option<Value> {
    self.map().get(key).copied()
  }

  #[inline]
  fn map(&self) -> &GcOrdHashMapN<Ref<Str>, Value> {
    unsafe { &*self.map.get() }
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  fn map_mut(&self) -> &mut GcOrdHashMapN<Ref<Str>, Value> {
    unsafe { &mut *self.map.get() }
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  fn map_alloc<'gc>(&self, gc: &'gc Gc) -> &mut GcOrdHashMap<'gc, Ref<Str>, Value> {
    let map = unsafe { &mut *self.map.get() };
    map.as_alloc_mut(gc)
  }
}

impl Object for Table {
  // We don't want to call `Drop` on the contents of the inner `table` or `vec`.
  // The `Table` object and its backing storage will be deallocated
  // by the GC at some point.
  const NEEDS_DROP: bool = false;
}
impl Debug for Table {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_map().entries(self.map().iter()).finish()
  }
}

impl Display for Table {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_map()
      .entries(
        self
          .map()
          .iter()
          .map(|(k, v)| (DelegateDebugToDisplay(k), DelegateDebugToDisplay(v))),
      )
      .finish()
  }
}

#[derive(Debug)]
pub struct TableDescriptor {
  start: Reg<u8>,
  keys: HashSet<Ref<Str>, BuildHasherDefault<FxHasher>, NoAlloc>,
}

impl TableDescriptor {
  pub fn try_new_in(gc: &Gc, start: Reg<u8>, keys: &[Ref<Str>]) -> Result<Ref<Self>, AllocError> {
    let mut k = GcHashSet::with_hasher_in(fx(), Alloc::new(gc));
    k.try_reserve(keys.len())?;
    k.extend(keys);
    let keys = k.to_no_alloc();
    gc.try_alloc(Self { start, keys })
  }

  #[inline]
  pub fn start(&self) -> Reg<u8> {
    self.start
  }

  #[inline]
  pub fn count(&self) -> u8 {
    self.keys.len() as u8
  }

  #[inline]
  pub fn keys(&self) -> &HashSet<Ref<Str>, BuildHasherDefault<FxHasher>, NoAlloc> {
    &self.keys
  }
}

impl Display for TableDescriptor {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<table>")
  }
}

impl Object for TableDescriptor {
  const NEEDS_DROP: bool = false;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn table_ops_new() {
    let gc = Gc::new();

    let table = Table::try_new_in(&gc).unwrap();
    assert_eq!(table.len(), 0);
    assert_eq!(table.capacity(), 0);
    table
      .try_insert(
        &gc,
        Str::try_new_in(&gc, "test").unwrap(),
        Value::new(10i32),
      )
      .unwrap();
    assert_eq!(table.len(), 1);
    assert!(table.capacity() > 0);
    assert_eq!(table.get("test").unwrap().cast::<i32>().unwrap(), 10i32);
    let value = table.remove("test").unwrap();
    assert_eq!(table.len(), 0);
    assert!(table.capacity() > 0);
    assert!(table.get("test").is_none());
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }

  #[test]
  fn table_ops_with_cap() {
    let gc = Gc::new();

    let table = Table::try_with_capacity_in(&gc, 1).unwrap();
    assert_eq!(table.len(), 0);
    assert!(table.capacity() > 0);
    table
      .try_insert(
        &gc,
        Str::try_new_in(&gc, "test").unwrap(),
        Value::new(10i32),
      )
      .unwrap();
    assert_eq!(table.len(), 1);
    assert!(table.capacity() > 0);
    assert_eq!(table.get("test").unwrap().cast::<i32>().unwrap(), 10i32);
    let value = table.remove("test").unwrap();
    assert_eq!(table.len(), 0);
    assert!(table.capacity() > 0);
    assert!(table.get("test").is_none());
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }
}
