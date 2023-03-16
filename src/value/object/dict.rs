//! Hebi's dictionary type.
//!
//! A dictionary is an ordered hashmap.

// NOTE: Before adding a method, think about whether or not it gives users
// mutable access to the keys of the map. It should not be possible.

use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Index, IndexMut, RangeBounds};

use indexmap::{map, Equivalent, IndexMap};

use super::{Access, Str, Value};
use crate::ctx::Context;
use crate::value::handle::Handle;
use crate::{public, Error, Result};

type Inner = IndexMap<Handle<Str>, Value>;

#[derive(Default)]
pub struct Dict {
  inner: Inner,
}

impl Dict {
  /// Create a new map. (Does not allocate.)
  pub fn new() -> Self {
    Self {
      inner: Inner::new(),
    }
  }

  /// Create a new map with capacity for `n` key-value pairs. (Does not
  /// allocate if `n` is zero.)
  ///
  /// Computes in **O(n)** time.
  pub fn with_capacity(n: usize) -> Self {
    Self {
      inner: Inner::with_capacity(n),
    }
  }
}

#[derive::delegate_to_handle]
impl Dict {
  pub fn capacity(&self) -> usize {
    self.inner.capacity()
  }

  /// Return the number of key-value pairs in the map.
  ///
  /// Computes in **O(1)** time.
  #[inline]
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  /// Returns true if the map contains no elements.
  ///
  /// Computes in **O(1)** time.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Return an iterator over the key-value pairs of the map, in their order
  pub fn iter(&self) -> map::Iter<'_, Handle<Str>, Value> {
    self.inner.iter()
  }

  /// Return an iterator over the key-value pairs of the map, in their order
  pub fn iter_mut(&mut self) -> map::IterMut<'_, Handle<Str>, Value> {
    self.inner.iter_mut()
  }

  /// Return an iterator over the keys of the map, in their order
  pub fn keys(&self) -> map::Keys<'_, Handle<Str>, Value> {
    self.inner.keys()
  }

  /// Return an iterator over the values of the map, in their order
  pub fn values(&self) -> map::Values<'_, Handle<Str>, Value> {
    self.inner.values()
  }

  /// Return an iterator over mutable references to the values of the map,
  /// in their order
  pub fn values_mut(&mut self) -> map::ValuesMut<'_, Handle<Str>, Value> {
    self.inner.values_mut()
  }

  /// Remove all key-value pairs in the map, while preserving its capacity.
  ///
  /// Computes in **O(n)** time.
  pub fn clear(&mut self) {
    self.inner.clear();
  }

  /// Shortens the map, keeping the first `len` elements and dropping the rest.
  ///
  /// If `len` is greater than the map's current length, this has no effect.
  pub fn truncate(&mut self, len: usize) {
    self.inner.truncate(len);
  }

  /// Clears the `IndexMap` in the given index range, returning those
  /// key-value pairs as a drain iterator.
  ///
  /// The range may be any type that implements `RangeBounds<usize>`,
  /// including all of the `std::ops::Range*` types, or even a tuple pair of
  /// `Bound` start and end values. To drain the map entirely, use `RangeFull`
  /// like `map.drain(..)`.
  ///
  /// This shifts down all entries following the drained range to fill the
  /// gap, and keeps the allocated memory for reuse.
  ///
  /// ***Panics*** if the starting point is greater than the end point or if
  /// the end point is greater than the length of the map.
  pub fn drain<R>(&mut self, range: R) -> map::Drain<'_, Handle<Str>, Value>
  where
    R: RangeBounds<usize>,
  {
    self.inner.drain(range)
  }

  /// Reserve capacity for `additional` more key-value pairs.
  ///
  /// Computes in **O(n)** time.
  pub fn reserve(&mut self, additional: usize) {
    self.inner.reserve(additional);
  }

  /// Shrink the capacity of the map as much as possible.
  ///
  /// Computes in **O(n)** time.
  pub fn shrink_to_fit(&mut self) {
    self.inner.shrink_to(0);
  }

  /// Shrink the capacity of the map with a lower limit.
  ///
  /// Computes in **O(n)** time.
  pub fn shrink_to(&mut self, min_capacity: usize) {
    self.inner.shrink_to(min_capacity);
  }

  /// Insert a key-value pair in the map.
  ///
  /// If an equivalent key already exists in the map: the key remains and
  /// retains in its place in the order, its corresponding value is updated
  /// with `value` and the older value is returned inside `Some(_)`.
  ///
  /// If no equivalent key existed in the map: the new key-value pair is
  /// inserted, last in order, and `None` is returned.
  ///
  /// Computes in **O(1)** time (amortized average).
  ///
  /// See also [`entry`](#method.entry) if you you want to insert *or* modify
  /// or if you need to get the index of the corresponding key-value pair.
  pub fn insert(&mut self, key: Handle<Str>, value: impl Into<Value>) -> Option<Value> {
    self.inner.insert(key, value.into())
  }

  /// Get the given key’s corresponding entry in the map for insertion and/or
  /// in-place manipulation.
  ///
  /// Computes in **O(1)** time (amortized average).
  pub fn entry(&mut self, key: Handle<Str>) -> map::Entry<'_, Handle<Str>, Value> {
    self.inner.entry(key)
  }

  /// Return `true` if an equivalent to `key` exists in the map.
  ///
  /// Computes in **O(1)** time (average).
  pub fn contains_key<Q>(&self, key: &Q) -> bool
  where
    Q: ?Sized + Hash + Equivalent<Handle<Str>>,
  {
    self.inner.contains_key(key)
  }

  /// Return a reference to the value stored for `key`, if it is present,
  /// else `None`.
  ///
  /// Computes in **O(1)** time (average).
  pub fn get<Q>(&self, key: &Q) -> Option<&Value>
  where
    Q: ?Sized + Hash + Equivalent<Handle<Str>>,
  {
    self.inner.get(key)
  }

  pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
  where
    Q: ?Sized + Hash + Equivalent<Handle<Str>>,
  {
    self.inner.remove(key)
  }

  /// Return references to the key-value pair stored for `key`,
  /// if it is present, else `None`.
  ///
  /// Computes in **O(1)** time (average).
  pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&str, &Value)>
  where
    Q: ?Sized + Hash + Equivalent<Handle<Str>>,
  {
    self.inner.get_key_value(key).map(|(k, v)| (k.as_str(), v))
  }

  pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
  where
    Q: ?Sized + Hash + Equivalent<Handle<Str>>,
  {
    self.inner.get_mut(key)
  }

  pub fn get_index(&self, index: usize) -> Option<(&str, &Value)> {
    self.inner.get_index(index).map(|(k, v)| (k.as_str(), v))
  }

  pub fn get_index_mut(&mut self, index: usize) -> Option<(&str, &mut Value)> {
    self
      .inner
      .get_index_mut(index)
      .map(|(k, v)| (k.as_str(), v))
  }

  /// Remove the last key-value pair
  ///
  /// This preserves the order of the remaining elements.
  ///
  /// Computes in **O(1)** time (average).
  pub fn pop(&mut self) -> Option<(Handle<Str>, Value)> {
    self.inner.pop()
  }

  /// Scan through each key-value pair in the map and keep those where the
  /// closure `keep` returns `true`.
  ///
  /// The elements are visited in order, and remaining elements keep their
  /// order.
  ///
  /// Computes in **O(n)** time (average).
  pub fn retain<F>(&mut self, keep: F)
  where
    F: FnMut(&Handle<Str>, &mut Value) -> bool,
  {
    self.inner.retain(keep);
  }

  /// Sort the map’s key-value pairs by the default ordering of the keys.
  ///
  /// See [`sort_by`](Self::sort_by) for details.
  pub fn sort_keys(&mut self) {
    self.inner.sort_keys()
  }

  /// Sort the map’s key-value pairs in place using the comparison
  /// function `cmp`.
  ///
  /// The comparison function receives two key and value pairs to compare (you
  /// can sort by keys or values or their combination as needed).
  ///
  /// Computes in **O(n log n + c)** time and **O(n)** space where *n* is
  /// the length of the map and *c* the capacity. The sort is stable.
  pub fn sort_by<F>(&mut self, cmp: F)
  where
    F: FnMut(&Handle<Str>, &Value, &Handle<Str>, &Value) -> Ordering,
  {
    self.inner.sort_by(cmp)
  }

  /// Sort the map's key-value pairs by the default ordering of the keys, but
  /// may not preserve the order of equal elements.
  ///
  /// See [`sort_unstable_by`](Self::sort_unstable_by) for details.
  pub fn sort_unstable_keys(&mut self) {
    self.inner.sort_unstable_keys()
  }

  /// Sort the map's key-value pairs in place using the comparison function
  /// `cmp`, but may not preserve the order of equal elements.
  ///
  /// The comparison function receives two key and value pairs to compare (you
  /// can sort by keys or values or their combination as needed).
  ///
  /// Computes in **O(n log n + c)** time where *n* is
  /// the length of the map and *c* is the capacity. The sort is unstable.
  pub fn sort_unstable_by<F>(&mut self, cmp: F)
  where
    F: FnMut(&Handle<Str>, &Value, &Handle<Str>, &Value) -> Ordering,
  {
    self.inner.sort_unstable_by(cmp)
  }

  /// Reverses the order of the map’s key-value pairs in place.
  ///
  /// Computes in **O(n)** time and **O(1)** space.
  pub fn reverse(&mut self) {
    self.inner.reverse()
  }
}

impl Access for Dict {
  fn is_frozen(&self) -> bool {
    false
  }

  fn field_get(&self, _: &Context, key: &str) -> Result<Option<Value>, Error> {
    Ok(match key {
      "len" => Some(Value::int(self.inner.len() as i32)),
      _ => None,
    })
  }

  fn index_get(&self, _: &Context, key: Value) -> Result<Option<Value>, Error> {
    Ok(match key.to_str() {
      Some(key) => self.inner.get(key.as_str()).cloned(),
      None => None,
    })
  }

  fn index_set(&mut self, _: &Context, key: Value, value: Value) -> Result<(), Error> {
    let Some(key) = key.clone().to_str() else {
      return Err(Error::runtime(format!("cannot index dictionary using {key}")))
    };
    self.inner.insert(key, value);
    Ok(())
  }
}

impl<'a> IntoIterator for &'a Dict {
  type Item = (&'a Handle<Str>, &'a Value);
  type IntoIter = map::Iter<'a, Handle<Str>, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.inner.iter()
  }
}

impl<'a> IntoIterator for &'a mut Dict {
  type Item = (&'a Handle<Str>, &'a mut Value);
  type IntoIter = map::IterMut<'a, Handle<Str>, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.inner.iter_mut()
  }
}

impl IntoIterator for Dict {
  type Item = (Handle<Str>, Value);

  type IntoIter = map::IntoIter<Handle<Str>, Value>;

  fn into_iter(self) -> Self::IntoIter {
    self.inner.into_iter()
  }
}

impl<Q> Index<&Q> for Dict
where
  Q: ?Sized + Hash + Equivalent<Handle<Str>>,
{
  type Output = Value;

  /// Returns a reference to the value corresponding to the supplied `key`.
  ///
  /// ***Panics*** if `key` is not present in the map.
  fn index(&self, index: &Q) -> &Self::Output {
    self.inner.index(index)
  }
}

impl<Q> IndexMut<&Q> for Dict
where
  Q: ?Sized + Hash + Equivalent<Handle<Str>>,
{
  /// Returns a mutable reference to the value corresponding to the supplied
  /// `key`.
  ///
  /// ***Panics*** if `key` is not present in the map.
  fn index_mut(&mut self, key: &Q) -> &mut Value {
    self.inner.index_mut(key)
  }
}

impl FromIterator<(Handle<Str>, Value)> for Dict {
  fn from_iter<T: IntoIterator<Item = (Handle<Str>, Value)>>(iter: T) -> Self {
    Self {
      inner: Inner::from_iter(iter),
    }
  }
}

impl<const N: usize> From<[(Handle<Str>, Value); N]> for Dict {
  fn from(arr: [(Handle<Str>, Value); N]) -> Self {
    Self::from_iter(arr)
  }
}

impl Extend<(Handle<Str>, Value)> for Dict {
  fn extend<T: IntoIterator<Item = (Handle<Str>, Value)>>(&mut self, iter: T) {
    self.inner.extend(iter)
  }
}

impl Display for Dict {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{")?;
    let mut iter = self.iter().peekable();
    while let Some((key, value)) = iter.next() {
      write!(f, "{key}: {value}")?;
      if iter.peek().is_some() {
        write!(f, ", ")?;
      }
    }
    write!(f, "}}")
  }
}

#[derive::builtin]
impl Dict {}
