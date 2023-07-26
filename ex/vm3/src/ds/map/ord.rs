use core::cmp;
use core::fmt::{Debug, Display};
use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use core::mem::replace;

use allocator_api2::alloc::Allocator;
use allocator_api2::vec::Vec;
use hashbrown::raw::RawTable;
use hashbrown::Equivalent;
use rustc_hash::FxHasher;

use crate::error::AllocError;
use crate::util::DelegateDebugToDisplay;

pub struct OrdHashMap<K, V, A: Allocator + Clone> {
  table: RawTable<usize, A>,
  vec: Vec<Entry<K, V>, A>,
  // TODO: feature for DOS-resistant hasher
  hash_builder: BuildHasherDefault<FxHasher>,
}

impl<K, V, A> OrdHashMap<K, V, A>
where
  A: Allocator + Clone,
{
  pub fn new_in(a: A) -> Self {
    let table = RawTable::new_in(a.clone());
    let vec = Vec::new_in(a);
    let hash_builder = BuildHasherDefault::default();
    OrdHashMap {
      table,
      vec,
      hash_builder,
    }
  }

  /// Allocates a new table with space for at least `capacity` entries.
  pub fn try_with_capacity_in(capacity: usize, a: A) -> Result<Self, AllocError> {
    let table = RawTable::try_with_capacity_in(capacity, a.clone())?;
    let vec = Vec::with_capacity_in(capacity, a);
    let hash_builder = BuildHasherDefault::default();

    Ok(OrdHashMap {
      table,
      vec,
      hash_builder,
    })
  }

  #[inline]
  pub fn allocators(&self) -> (&A, &A) {
    (self.table.allocator(), self.vec.allocator())
  }

  /// Returns the number of entries currently in the table.
  #[inline]
  pub fn len(&self) -> usize {
    self.table.len()
  }

  /// Returns the table's remaining capacity.
  #[inline]
  pub fn capacity(&self) -> usize {
    cmp::min(self.table.capacity(), self.vec.capacity())
  }

  /// Returns `true` if the table is empty.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.table.is_empty()
  }
}

impl<K, V, A> OrdHashMap<K, V, A>
where
  K: Hash + Eq,
  A: Allocator + Clone,
{
  #[inline]
  pub fn insert(&mut self, key: K, value: V) -> Option<V> {
    self.try_insert(key, value).unwrap()
  }

  /// Inserts `value` into the table associated with the key `key`.
  pub fn try_insert(&mut self, key: K, value: V) -> Result<Option<V>, AllocError> {
    self.try_reserve(1)?;
    Ok(unsafe { self.try_insert_no_grow(key, value).unwrap_unchecked() })
  }

  /// Removes `key` from the table, returning it if it exists.
  pub fn remove<Q: ?Sized + Hash + Equivalent<K>>(&mut self, key: &Q) -> Option<V> {
    let hash = self.hash(key);
    let eq = |i: &usize| unsafe { key.equivalent(&self.vec.get_unchecked(*i).key) };
    match self.table.remove_entry(hash, eq) {
      Some(index) => {
        let entry = self.vec.swap_remove(index);
        if let Some(entry) = self.vec.get(index) {
          let last = self.vec.len();
          let current = unsafe {
            self
              .table
              .get_mut(entry.hash, |i| *i == last)
              .unwrap_unchecked()
          };
          *current = index;
        }
        Some(entry.value)
      }
      None => None,
    }
  }

  /// Like [`Table::try_insert`], but will return the `(key, value)` pair
  /// if the table does not have enough spare capacity.
  pub fn try_insert_no_grow(&mut self, key: K, value: V) -> Result<Option<V>, (K, V)> {
    let hash = self.hash(&key);
    match self.table.find_or_find_insert_slot(
      hash,
      // The indices are guaranteed to exist in `vec`.
      |i| &key == unsafe { &self.vec.get_unchecked(*i).key },
      |i| unsafe { self.vec.get_unchecked(*i) }.hash,
    ) {
      Ok(bucket) => {
        // The pointer in the bucket is valid for reads.
        let index = unsafe { bucket.as_ptr().read() };
        // The index is guaranteed to exist.
        let prev = replace(
          &mut unsafe { self.vec.get_unchecked_mut(index) }.value,
          value,
        );
        Ok(Some(prev))
      }
      Err(slot) => {
        let index = self.vec.len();
        let entry = Entry { hash, key, value };
        match self.vec.push_within_capacity(entry) {
          Ok(()) => {
            unsafe { self.table.insert_in_slot(hash, slot, index) };
            Ok(None)
          }
          Err(entry) => Err((entry.key, entry.value)),
        }
      }
    }
  }

  #[inline]
  pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
    self
      .table
      // The index is guaranteed to exist
      .try_reserve(additional, |i| unsafe { self.vec.get_unchecked(*i) }.hash)?;
    self.vec.try_reserve(additional).map_err(|_| AllocError)?;

    Ok(())
  }

  pub fn get<Q: ?Sized + Hash + Equivalent<K>>(&self, key: &Q) -> Option<&V> {
    let hash = self.hash(key);
    // The indices are guaranteed to exist
    let eq = |i: &usize| unsafe { key.equivalent(&self.vec.get_unchecked(*i).key) };
    self
      .table
      .get(hash, eq)
      .map(|index| unsafe { &self.vec.get_unchecked(*index).value })
  }

  #[inline]
  pub fn get_index(&self, index: usize) -> Option<&V> {
    match self.vec.get(index) {
      Some(entry) => Some(&entry.value),
      None => None,
    }
  }

  #[inline]
  pub fn set_index(&mut self, index: usize, value: V) -> bool {
    match self.vec.get_mut(index) {
      Some(slot) => {
        slot.value = value;
        true
      }
      None => false,
    }
  }

  #[inline]
  pub fn contains_key<Q: ?Sized + Hash + Equivalent<K>>(&self, key: &Q) -> bool {
    let hash = self.hash(key);
    // The indices are guaranteed to exist
    let eq = |i: &usize| unsafe { key.equivalent(&self.vec.get_unchecked(*i).key) };
    self.table.get(hash, eq).is_some()
  }

  #[inline]
  fn hash<Q: ?Sized + Hash>(&self, key: &Q) -> u64 {
    let mut hasher = self.hash_builder.build_hasher();
    key.hash(&mut hasher);
    hasher.finish()
  }
}

impl<K, V, A: Allocator + Clone> OrdHashMap<K, V, A> {
  pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
    self.vec.iter().map(|e| (&e.key, &e.value))
  }
}

impl<K, V, A> Default for OrdHashMap<K, V, A>
where
  K: Hash + Eq,
  A: Default + Allocator + Clone,
{
  fn default() -> Self {
    Self::new_in(A::default())
  }
}

struct Entry<K, V> {
  hash: u64,
  key: K,
  value: V,
}

impl<K: Debug, V: Debug, A: Allocator + Clone> Debug for OrdHashMap<K, V, A> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_map().entries(self.iter()).finish()
  }
}

impl<K: Display, V: Display, A: Allocator + Clone> Display for OrdHashMap<K, V, A> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_map()
      .entries(
        self
          .iter()
          .map(|(k, v)| (DelegateDebugToDisplay(k), DelegateDebugToDisplay(v))),
      )
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use allocator_api2::alloc::Global;

  use super::*;

  #[test]
  fn map_ops_new() {
    let mut map = OrdHashMap::new_in(Global);
    assert_eq!(map.len(), 0);
    assert_eq!(map.capacity(), 0);
    map.try_insert("test", 10i32).unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.capacity() > 0);
    assert_eq!(map.get("test").unwrap(), &10i32);
    let value = map.remove("test").unwrap();
    assert_eq!(map.len(), 0);
    assert!(map.capacity() > 0);
    assert!(map.get("test").is_none());
    assert_eq!(value, 10i32);
  }

  #[test]
  fn table_ops_with_cap() {
    let mut map = OrdHashMap::try_with_capacity_in(1, Global).unwrap();
    assert_eq!(map.len(), 0);
    assert!(map.capacity() > 0);
    map.try_insert("test", 10i32).unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.capacity() > 0);
    assert_eq!(map.get("test").unwrap(), &10i32);
    let value = map.remove("test").unwrap();
    assert_eq!(map.len(), 0);
    assert!(map.capacity() > 0);
    assert!(map.get("test").is_none());
    assert_eq!(value, 10i32);
  }
}
