//! Hebi's Table type.
//!
//! [`Table`] is implemented as an ordered hash map.
//! It is _not_ resistant to HashDOS by default.
//!
//! Implementation is heavily inspired by [indexmap](https://docs.rs/indexmap/latest/indexmap/).

use core::cell::UnsafeCell;
use core::fmt::{Debug, Display};

use super::string::Str;
use crate::ds::map::{GcOrdHashMap, GcOrdHashMapN};
use crate::ds::set::{GcHashSet, GcHashSetN};
use crate::ds::{fx, HasAlloc, HasNoAlloc};
use crate::error::AllocError;
use crate::gc::{Alloc, Gc, Object, Ref, NO_ALLOC};
use crate::op::Reg;
use crate::util::DelegateDebugToDisplay;
use crate::val::Value;

pub struct Map {
  map: UnsafeCell<GcOrdHashMapN<Ref<Str>, Value>>,
}

impl Map {
  /// Allocates a new map.
  ///
  /// The map is initially empty.
  /// The only allocation done here is to put the `Map` object
  /// onto the garbage-collected heap.
  pub fn new(gc: &Gc) -> Result<Ref<Self>, AllocError> {
    let map = UnsafeCell::new(GcOrdHashMapN::new_in(NO_ALLOC));
    gc.try_alloc(Map { map })
  }

  /// Allocates a new map with space for at least `capacity` entries.
  pub fn try_with_capacity_in(gc: &Gc, capacity: usize) -> Result<Ref<Self>, AllocError> {
    let map = GcOrdHashMap::try_with_capacity_in(capacity, Alloc::new(gc))?;
    let map = UnsafeCell::new(map.to_no_alloc());
    gc.try_alloc(Map { map })
  }

  /// Returns the number of entries currently inhabiting the map.
  #[inline]
  pub fn len(&self) -> usize {
    self.map().len()
  }

  /// Returns the map's remaining capacity.
  #[inline]
  pub fn capacity(&self) -> usize {
    self.map().capacity()
  }

  /// Returns `true` if the map is empty.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.map().is_empty()
  }

  /// Inserts `value` into the map associated with the key `key`.
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

  /// Removes `key` from the map, returning it if it exists.
  pub fn remove(&self, key: &str) -> Option<Value> {
    self.map_mut().remove(key)
  }

  /// Like [`Map::try_insert`], but will return the `(key, value)` pair
  /// if the map does not have enough spare capacity.
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

  #[inline]
  pub fn get(&self, key: &str) -> Option<Value> {
    self.map().get(key).copied()
  }

  #[inline]
  pub fn get_index(&self, index: usize) -> Option<Value> {
    self.map().get_index(index).copied()
  }

  #[inline]
  pub fn set_index(&self, index: usize, value: Value) -> bool {
    self.map_mut().set_index(index, value)
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

impl Object for Map {
  // We don't want to call `Drop` on the contents of the inner `map` or `vec`.
  // The `Map` object and its backing storage will be deallocated
  // by the GC at some point.
  const NEEDS_DROP: bool = false;
}
impl Debug for Map {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_map().entries(self.map().iter()).finish()
  }
}

impl Display for Map {
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
pub struct MapProto {
  start: Reg<u8>,
  keys: GcHashSetN<Ref<Str>>,
}

impl MapProto {
  pub fn new(gc: &Gc, start: Reg<u8>, keys: &[Ref<Str>]) -> Result<Ref<Self>, AllocError> {
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
  pub fn keys(&self) -> &GcHashSetN<Ref<Str>> {
    &self.keys
  }
}

impl Display for MapProto {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<map>")
  }
}

impl Object for MapProto {
  const NEEDS_DROP: bool = false;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn table_ops_new() {
    let gc = Gc::new();

    let map = Map::new(&gc).unwrap();
    assert_eq!(map.len(), 0);
    assert_eq!(map.capacity(), 0);
    map
      .try_insert(&gc, Str::new(&gc, "test").unwrap(), Value::new(10i32))
      .unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.capacity() > 0);
    assert_eq!(map.get("test").unwrap().cast::<i32>().unwrap(), 10i32);
    let value = map.remove("test").unwrap();
    assert_eq!(map.len(), 0);
    assert!(map.capacity() > 0);
    assert!(map.get("test").is_none());
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }

  #[test]
  fn table_ops_with_cap() {
    let gc = Gc::new();

    let map = Map::try_with_capacity_in(&gc, 1).unwrap();
    assert_eq!(map.len(), 0);
    assert!(map.capacity() > 0);
    map
      .try_insert(&gc, Str::new(&gc, "test").unwrap(), Value::new(10i32))
      .unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.capacity() > 0);
    assert_eq!(map.get("test").unwrap().cast::<i32>().unwrap(), 10i32);
    let value = map.remove("test").unwrap();
    assert_eq!(map.len(), 0);
    assert!(map.capacity() > 0);
    assert!(map.get("test").is_none());
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }
}
