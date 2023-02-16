//! Mu's dictionary type.
//!
//! A dictionary is an ordered hashmap.

// NOTE: Before adding a method, think about whether or not it gives users
// mutable access to the keys of the map. It should not be possible.

use std::cmp::Ordering;
use std::collections::hash_map::RandomState;
use std::hash::Hash;
use std::ops::{Index, IndexMut, RangeBounds};

use beef::lean::Cow;
use indexmap::{map, Equivalent, IndexMap};

use super::handle::Handle;
use super::{Object, Ptr, Str, Value};
use crate::value::ptr::Ref;

type Inner = IndexMap<Key, Value>;

#[derive(Clone, Default)]
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

  pub fn capacity(&self) -> usize {
    self.inner.capacity()
  }

  pub fn hasher(&self) -> &RandomState {
    self.inner.hasher()
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
  pub fn iter(&self) -> map::Iter<'_, Key, Value> {
    self.inner.iter()
  }

  /// Return an iterator over the key-value pairs of the map, in their order
  pub fn iter_mut(&mut self) -> map::IterMut<'_, Key, Value> {
    self.inner.iter_mut()
  }

  /// Return an iterator over the keys of the map, in their order
  pub fn keys(&self) -> map::Keys<'_, Key, Value> {
    self.inner.keys()
  }

  /// Return an owning iterator over the keys of the map, in their order
  pub fn into_keys(self) -> map::IntoKeys<Key, Value> {
    self.inner.into_keys()
  }

  /// Return an iterator over the values of the map, in their order
  pub fn values(&self) -> map::Values<'_, Key, Value> {
    self.inner.values()
  }

  /// Return an iterator over mutable references to the values of the map,
  /// in their order
  pub fn values_mut(&mut self) -> map::ValuesMut<'_, Key, Value> {
    self.inner.values_mut()
  }

  /// Return an owning iterator over the values of the map, in their order
  pub fn into_values(self) -> map::IntoValues<Key, Value> {
    self.inner.into_values()
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
  pub fn drain<R>(&mut self, range: R) -> map::Drain<'_, Key, Value>
  where
    R: RangeBounds<usize>,
  {
    self.inner.drain(range)
  }

  /// Splits the collection into two at the given index.
  ///
  /// Returns a newly allocated map containing the elements in the range
  /// `[at, len)`. After the call, the original map will be left containing
  /// the elements `[0, at)` with its previous capacity unchanged.
  ///
  /// ***Panics*** if `at > len`.
  pub fn split_off(&mut self, at: usize) -> Self {
    Dict {
      inner: self.inner.split_off(at),
    }
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
  pub fn insert(&mut self, key: impl Into<Key>, value: impl Into<Value>) -> Option<Value> {
    self.inner.insert(key.into(), value.into())
  }

  /// Get the given key’s corresponding entry in the map for insertion and/or
  /// in-place manipulation.
  ///
  /// Computes in **O(1)** time (amortized average).
  pub fn entry(&mut self, key: impl Into<Key>) -> map::Entry<'_, Key, Value> {
    self.inner.entry(key.into())
  }

  /// Return `true` if an equivalent to `key` exists in the map.
  ///
  /// Computes in **O(1)** time (average).
  pub fn contains_key<Q>(&self, key: &Q) -> bool
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.inner.contains_key(key)
  }

  /// Return a reference to the value stored for `key`, if it is present,
  /// else `None`.
  ///
  /// Computes in **O(1)** time (average).
  pub fn get<Q>(&self, key: &Q) -> Option<&Value>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.inner.get(key)
  }

  pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.inner.remove(key)
  }

  /// Return references to the key-value pair stored for `key`,
  /// if it is present, else `None`.
  ///
  /// Computes in **O(1)** time (average).
  pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&Key, &Value)>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.inner.get_key_value(key)
  }

  pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.inner.get_mut(key)
  }

  /// Remove the last key-value pair
  ///
  /// This preserves the order of the remaining elements.
  ///
  /// Computes in **O(1)** time (average).
  pub fn pop(&mut self) -> Option<(Key, Value)> {
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
    F: FnMut(&Key, &mut Value) -> bool,
  {
    self.inner.retain(keep);
  }

  /// Sort the map’s key-value pairs by the default ordering of the keys.
  ///
  /// See [`sort_by`](Self::sort_by) for details.
  pub fn sort_keys(&mut self)
  where
    Key: Ord,
  {
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
    F: FnMut(&Key, &Value, &Key, &Value) -> Ordering,
  {
    self.inner.sort_by(cmp)
  }

  /// Sort the key-value pairs of the map and return a by-value iterator of
  /// the key-value pairs with the result.
  ///
  /// The sort is stable.
  pub fn sorted_by<F>(self, cmp: F) -> map::IntoIter<Key, Value>
  where
    F: FnMut(&Key, &Value, &Key, &Value) -> Ordering,
  {
    self.inner.sorted_by(cmp)
  }

  /// Sort the map's key-value pairs by the default ordering of the keys, but
  /// may not preserve the order of equal elements.
  ///
  /// See [`sort_unstable_by`](Self::sort_unstable_by) for details.
  pub fn sort_unstable_keys(&mut self)
  where
    Key: Ord,
  {
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
    F: FnMut(&Key, &Value, &Key, &Value) -> Ordering,
  {
    self.inner.sort_unstable_by(cmp)
  }

  /// Sort the key-value pairs of the map and return a by-value iterator of
  /// the key-value pairs with the result.
  ///
  /// The sort is unstable.
  #[inline]
  pub fn sorted_unstable_by<F>(self, cmp: F) -> map::IntoIter<Key, Value>
  where
    F: FnMut(&Key, &Value, &Key, &Value) -> Ordering,
  {
    self.inner.sorted_unstable_by(cmp)
  }

  /// Reverses the order of the map’s key-value pairs in place.
  ///
  /// Computes in **O(n)** time and **O(1)** space.
  pub fn reverse(&mut self) {
    self.inner.reverse()
  }
}

#[derive(Clone)]
pub struct Key(pub(crate) KeyRepr);

impl Key {
  pub fn as_str(&self) -> Option<Ref<'_, str>> {
    match &self.0 {
      KeyRepr::String(v) => Some(Ref::map(v.borrow(), |v| v.as_str())),
      KeyRepr::Int(_) => None,
    }
  }

  pub(crate) fn write_to_string(&self, s: &mut String) {
    use std::fmt::Write;
    match &self.0 {
      KeyRepr::Int(v) => write!(s, "{v}").unwrap(),
      KeyRepr::String(v) => write!(s, "{}", v.borrow()).unwrap(),
    }
  }
}

/// The dictionary key type.
///
/// Maybe an integer or a string.
#[repr(align(16))]
#[derive(Clone)]
pub(crate) enum KeyRepr {
  Int(i32),
  /// This variant always contains a string.
  ///
  /// The only way to create this variant is via `TryFrom<Value>`, which rejects
  /// anything that is not a string.
  String(Handle<Str>),
}

impl From<i32> for Key {
  fn from(value: i32) -> Self {
    Key(KeyRepr::Int(value))
  }
}

impl<'a> From<&'a str> for Key {
  fn from(value: &'a str) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key(KeyRepr::String(unsafe {
      Handle::from_ptr_unchecked(Ptr::new(Object::str(value)))
    }))
  }
}

impl<'a> From<Cow<'a, str>> for Key {
  fn from(value: Cow<'a, str>) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key(KeyRepr::String(unsafe {
      Handle::from_ptr_unchecked(Ptr::new(Object::str(value.to_string())))
    }))
  }
}

impl From<Str> for Key {
  fn from(value: Str) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key(KeyRepr::String(unsafe {
      Handle::from_ptr_unchecked(Ptr::new(Object::str(value)))
    }))
  }
}

impl TryFrom<Value> for Key {
  type Error = InvalidKeyType;

  fn try_from(value: Value) -> Result<Self, Self::Error> {
    Handle::from_value(value)
      .map(|v| Key(KeyRepr::String(v)))
      .ok_or(InvalidKeyType)
  }
}

impl Equivalent<Key> for str {
  fn equivalent(&self, key: &Key) -> bool {
    match &key.0 {
      KeyRepr::Int(_) => false,
      KeyRepr::String(v) => v.borrow().as_str() == self,
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct InvalidKeyType;

impl std::fmt::Display for InvalidKeyType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "invalid key type")
  }
}

impl std::error::Error for InvalidKeyType {}

impl PartialEq for Key {
  fn eq(&self, other: &Self) -> bool {
    match (&self.0, &other.0) {
      (KeyRepr::Int(a), KeyRepr::Int(b)) => a == b,
      (KeyRepr::String(a), KeyRepr::String(b)) => a.borrow().as_str() == b.borrow().as_str(),
      _ => false,
    }
  }
}

impl Eq for Key {}

impl PartialOrd for Key {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    match (&self.0, &other.0) {
      (KeyRepr::Int(a), KeyRepr::Int(b)) => a.partial_cmp(b),
      (KeyRepr::Int(_), KeyRepr::String(_)) => Some(std::cmp::Ordering::Less),
      (KeyRepr::String(_), KeyRepr::Int(_)) => Some(std::cmp::Ordering::Greater),
      (KeyRepr::String(a), KeyRepr::String(b)) => {
        a.borrow().as_str().partial_cmp(b.borrow().as_str())
      }
    }
  }
}

impl Ord for Key {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    unsafe { self.partial_cmp(other).unwrap_unchecked() }
  }
}

impl Hash for Key {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match &self.0 {
      KeyRepr::Int(v) => v.hash(state),
      KeyRepr::String(v) => v.borrow().hash(state),
    }
  }
}

impl std::fmt::Debug for Key {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.0 {
      KeyRepr::Int(v) => std::fmt::Debug::fmt(v, f),
      KeyRepr::String(v) => std::fmt::Debug::fmt(v, f),
    }
  }
}

impl std::fmt::Display for Key {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.0 {
      KeyRepr::Int(v) => std::fmt::Display::fmt(v, f),
      KeyRepr::String(v) => std::fmt::Display::fmt(v, f),
    }
  }
}

impl std::fmt::Debug for Dict {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(&self.inner, f)
  }
}

impl<'a> IntoIterator for &'a Dict {
  type Item = (&'a Key, &'a Value);
  type IntoIter = map::Iter<'a, Key, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.inner.iter()
  }
}

impl<'a> IntoIterator for &'a mut Dict {
  type Item = (&'a Key, &'a mut Value);
  type IntoIter = map::IterMut<'a, Key, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.inner.iter_mut()
  }
}

impl IntoIterator for Dict {
  type Item = (Key, Value);

  type IntoIter = map::IntoIter<Key, Value>;

  fn into_iter(self) -> Self::IntoIter {
    self.inner.into_iter()
  }
}

impl<Q: ?Sized> Index<&Q> for Dict
where
  Q: Hash + Equivalent<Key>,
{
  type Output = Value;

  /// Returns a reference to the value corresponding to the supplied `key`.
  ///
  /// ***Panics*** if `key` is not present in the map.
  fn index(&self, index: &Q) -> &Self::Output {
    self.inner.index(index)
  }
}

impl<Q: ?Sized> IndexMut<&Q> for Dict
where
  Q: Hash + Equivalent<Key>,
{
  /// Returns a mutable reference to the value corresponding to the supplied
  /// `key`.
  ///
  /// ***Panics*** if `key` is not present in the map.
  fn index_mut(&mut self, key: &Q) -> &mut Value {
    self.inner.index_mut(key)
  }
}

impl FromIterator<(Key, Value)> for Dict {
  fn from_iter<T: IntoIterator<Item = (Key, Value)>>(iter: T) -> Self {
    Self {
      inner: Inner::from_iter(iter),
    }
  }
}

impl<const N: usize> From<[(Key, Value); N]> for Dict {
  fn from(arr: [(Key, Value); N]) -> Self {
    Self::from_iter(arr)
  }
}

impl Extend<(Key, Value)> for Dict {
  fn extend<T: IntoIterator<Item = (Key, Value)>>(&mut self, iter: T) {
    self.inner.extend(iter)
  }
}
