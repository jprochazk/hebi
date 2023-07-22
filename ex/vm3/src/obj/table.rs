//! Hebi's Table type.
//!
//! [`Table`] is implemented as an ordered hash map.
//! It is _not_ resistant to HashDOS by default.
//!
//! Implementation is heavily inspired by [indexmap](https://docs.rs/indexmap/latest/indexmap/).

use core::cell::UnsafeCell;
use core::cmp;
use core::fmt::{Debug, Display};
use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use core::mem::{replace, transmute};

use allocator_api2::vec::Vec;
use bumpalo::AllocErr;
use hashbrown::raw::RawTable;
use hashbrown::HashSet;
use rustc_hash::FxHasher;

use super::string::Str;
use crate::gc::{Alloc, Gc, NoAlloc, Object, Ref, NO_ALLOC};
use crate::op::Reg;
use crate::util::DelegateDebugToDisplay;
use crate::val::Value;

pub struct Table {
  table: UnsafeCell<RawTable<usize, NoAlloc>>,
  vec: UnsafeCell<Vec<Entry, NoAlloc>>,
  // TODO: feature for DOS-resistant hasher
  hash_builder: BuildHasherDefault<FxHasher>,
}

impl Table {
  /// Allocates a new table.
  ///
  /// The table is initially empty.
  /// The only allocation done here is to put the `Table` object
  /// onto the garbage-collected heap.
  pub fn try_new_in(gc: &Gc) -> Result<Ref<Self>, AllocErr> {
    let table = UnsafeCell::new(RawTable::new_in(NO_ALLOC));
    let vec = UnsafeCell::new(Vec::new_in(NO_ALLOC));
    let hash_builder = BuildHasherDefault::default();
    gc.try_alloc(Table {
      table,
      vec,
      hash_builder,
    })
  }

  /// Allocates a new table with space for at least `capacity` entries.
  pub fn try_with_capacity_in(gc: &Gc, capacity: usize) -> Result<Ref<Self>, AllocErr> {
    let table = RawTable::try_with_capacity_in(capacity, Alloc::new(gc)).map_err(|_| AllocErr)?;
    // We promise never to attempt to grow a `NoAlloc` table, so this is safe.
    let table = unsafe { transmute::<RawTable<usize, _>, RawTable<usize, NoAlloc>>(table) };

    let mut vec = Vec::new_in(Alloc::new(gc));
    vec.try_reserve_exact(capacity).map_err(|_| AllocErr)?;
    // We promise never to attempt to grow a `NoAlloc` table, so this is safe.
    let vec = unsafe { transmute::<Vec<Entry, _>, Vec<Entry, NoAlloc>>(vec) };

    let hash_builder = BuildHasherDefault::default();

    gc.try_alloc(Table {
      table: UnsafeCell::new(table),
      vec: UnsafeCell::new(vec),
      hash_builder,
    })
  }

  /// Returns the number of entries currently in the table.
  #[inline]
  pub fn len(&self) -> usize {
    unsafe { self.get_table() }.len()
  }

  /// Returns the table's remaining capacity.
  #[inline]
  pub fn capacity(&self) -> usize {
    let vec_cap = unsafe { self.get_vec().capacity() };
    let table_cap = unsafe { self.get_table().capacity() };
    cmp::min(vec_cap, table_cap)
  }

  /// Returns `true` if the table is empty.
  #[inline]
  pub fn is_empty(&self) -> bool {
    unsafe { self.get_table() }.is_empty()
  }

  /// Inserts `value` into the table associated with the key `key`.
  pub fn try_insert(
    &self,
    gc: &Gc,
    key: Ref<Str>,
    value: Value,
  ) -> Result<Option<Value>, AllocErr> {
    self.try_reserve(gc, 1)?;
    Ok(unsafe { self.try_insert_no_grow(key, value).unwrap_unchecked() })
  }

  /// Removes `key` from the table, returning it if it exists.
  pub fn remove(&self, key: &str) -> Option<Value> {
    let hash = self.hash(key);
    let vec = unsafe { self.get_vec_mut_no_alloc() };
    let table = unsafe { self.get_table_mut_no_alloc() };
    let eq = |i: &usize| unsafe { vec.get_unchecked(*i) }.key.as_str() == key;
    match table.remove_entry(hash, eq) {
      Some(index) => {
        let entry = vec.swap_remove(index);
        if let Some(entry) = vec.get(index) {
          let last = vec.len();
          let current = unsafe { table.get_mut(entry.hash, |i| *i == last).unwrap_unchecked() };
          *current = index;
        }
        Some(entry.value)
      }
      None => None,
    }
  }

  /// Like [`Table::try_insert`], but will return the `(key, value)` pair
  /// if the table does not have enough spare capacity.
  pub fn try_insert_no_grow(
    &self,
    key: Ref<Str>,
    value: Value,
  ) -> Result<Option<Value>, (Ref<Str>, Value)> {
    let hash = self.hash(key.as_str());
    let vec = unsafe { self.get_vec_mut_no_alloc() };
    let table = unsafe { self.get_table_mut_no_alloc() };
    match table.find_or_find_insert_slot(
      hash,
      // The indices are guaranteed to exist in `vec`.
      |i| unsafe { vec.get_unchecked(*i) }.key.as_str() == key.as_str(),
      |i| unsafe { vec.get_unchecked(*i) }.hash,
    ) {
      Ok(bucket) => {
        // The pointer in the bucket is valid for reads.
        let index = unsafe { bucket.as_ptr().read() };
        // The index is guaranteed to exist.
        let prev = replace(&mut unsafe { vec.get_unchecked_mut(index) }.value, value);
        Ok(Some(prev))
      }
      Err(slot) => {
        let index = vec.len();
        let entry = Entry { hash, key, value };
        match vec.push_within_capacity(entry) {
          Ok(()) => {
            unsafe { table.insert_in_slot(hash, slot, index) };
            Ok(None)
          }
          Err(entry) => Err((entry.key, entry.value)),
        }
      }
    }
  }

  #[inline]
  pub fn try_reserve(&self, gc: &Gc, additional: usize) -> Result<(), AllocErr> {
    let table = unsafe { self.get_table_mut_alloc(gc) };
    let vec = unsafe { self.get_vec_mut_alloc(gc) };

    table
      // The index is guaranteed to exist
      .try_reserve(additional, |i| unsafe { vec.get_unchecked(*i) }.hash)
      .map_err(|_| AllocErr)?;
    vec.try_reserve(additional).map_err(|_| AllocErr)?;

    Ok(())
  }

  pub fn get(&self, key: &str) -> Option<Value> {
    let hash = self.hash(key);
    let table = unsafe { self.get_table() };
    let vec = unsafe { self.get_vec() };
    // The indices are guaranteed to exist
    let eq = |i: &usize| unsafe { vec.get_unchecked(*i) }.key.as_str() == key;
    table
      .get(hash, eq)
      .map(|index| unsafe { vec.get_unchecked(*index).value })
  }

  #[inline]
  fn hash(&self, key: &str) -> u64 {
    let mut hasher = self.hash_builder.build_hasher();
    key.hash(&mut hasher);
    hasher.finish()
  }

  #[inline]
  unsafe fn get_table(&self) -> &RawTable<usize, NoAlloc> {
    // `get()` returns a possibly null pointer, which is never null here.
    self.table.get().as_ref().unwrap_unchecked()
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  unsafe fn get_table_mut_no_alloc(&self) -> &mut RawTable<usize, NoAlloc> {
    // `get()` returns a possibly null pointer, which is never null here.
    self.table.get().as_mut().unwrap_unchecked()
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  unsafe fn get_table_mut_alloc<'gc>(&self, gc: &'gc Gc) -> &mut RawTable<usize, Alloc<'gc>> {
    let table = self.get_table_mut_no_alloc();
    let table = transmute::<_, &mut RawTable<usize, Alloc<'gc>>>(table);
    table.allocator().set(gc);
    table
  }

  #[inline]
  unsafe fn get_vec(&self) -> &Vec<Entry, NoAlloc> {
    self.vec.get().as_ref().unwrap_unchecked()
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  unsafe fn get_vec_mut_no_alloc(&self) -> &mut Vec<Entry, NoAlloc> {
    self.vec.get().as_mut().unwrap_unchecked()
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  unsafe fn get_vec_mut_alloc<'gc>(&self, gc: &'gc Gc) -> &mut Vec<Entry, Alloc<'gc>> {
    let vec = self.get_vec_mut_no_alloc();
    let vec = transmute::<_, &mut Vec<Entry, Alloc<'gc>>>(vec);
    vec.allocator().set(gc);
    vec
  }
}

struct Entry {
  hash: u64,
  key: Ref<Str>,
  value: Value,
}

impl Object for Table {
  // We don't want to call `Drop` on the contents of the inner `table` or `vec`.
  // The `Table` object and its backing storage will be deallocated
  // by the GC at some point.
  const NEEDS_DROP: bool = false;
}

impl Debug for Table {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut f = f.debug_map();
    for entry in unsafe { self.get_vec() }.iter() {
      f.entry(&entry.key, &entry.value);
    }
    f.finish()
  }
}

impl Display for Table {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut f = f.debug_map();
    for entry in unsafe { self.get_vec() }.iter() {
      f.entry(
        &DelegateDebugToDisplay(entry.key),
        &DelegateDebugToDisplay(entry.value),
      );
    }
    f.finish()
  }
}

#[derive(Debug)]
pub struct TableDescriptor {
  start: Reg<u8>,
  keys: HashSet<Ref<Str>, BuildHasherDefault<FxHasher>, NoAlloc>,
}

impl TableDescriptor {
  pub fn try_new_in(gc: &Gc, start: Reg<u8>, keys: &[Ref<Str>]) -> Result<Ref<Self>, AllocErr> {
    let mut key_set: HashSet<Ref<Str>, BuildHasherDefault<FxHasher>, _> = HashSet::with_hasher_in(
      BuildHasherDefault::default(),
      Alloc::new(unsafe { &*(gc as *const _) }),
    );
    key_set.try_reserve(keys.len()).map_err(|_| AllocErr)?;
    key_set.extend(keys);

    let keys = unsafe { transmute::<_, HashSet<_, _, NoAlloc>>(key_set) };
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
