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
use super::{Access, Object, Ptr, Str, Value};

type Inner = IndexMap<StaticKey, Value>;

#[derive(Clone, Default)]
pub struct Dict {
  inner: Inner,
}

impl Access for Dict {
  fn is_frozen(&self) -> bool {
    false
  }

  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    Ok(match key.as_str() {
      Some("len") => Some((self.inner.len() as i32).into()),
      _ => None,
    })
  }

  fn index_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    Ok(self.inner.get(key).cloned())
  }

  fn index_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::Error> {
    self.inner.insert(key, value);
    Ok(())
  }
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
  pub fn iter(&self) -> map::Iter<'_, StaticKey, Value> {
    self.inner.iter()
  }

  /// Return an iterator over the key-value pairs of the map, in their order
  pub fn iter_mut(&mut self) -> map::IterMut<'_, StaticKey, Value> {
    self.inner.iter_mut()
  }

  /// Return an iterator over the keys of the map, in their order
  pub fn keys(&self) -> map::Keys<'_, StaticKey, Value> {
    self.inner.keys()
  }

  /// Return an owning iterator over the keys of the map, in their order
  pub fn into_keys(self) -> map::IntoKeys<StaticKey, Value> {
    self.inner.into_keys()
  }

  /// Return an iterator over the values of the map, in their order
  pub fn values(&self) -> map::Values<'_, StaticKey, Value> {
    self.inner.values()
  }

  /// Return an iterator over mutable references to the values of the map,
  /// in their order
  pub fn values_mut(&mut self) -> map::ValuesMut<'_, StaticKey, Value> {
    self.inner.values_mut()
  }

  /// Return an owning iterator over the values of the map, in their order
  pub fn into_values(self) -> map::IntoValues<StaticKey, Value> {
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
  pub fn drain<R>(&mut self, range: R) -> map::Drain<'_, StaticKey, Value>
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
  pub fn insert(&mut self, key: impl Into<StaticKey>, value: impl Into<Value>) -> Option<Value> {
    self.inner.insert(key.into(), value.into())
  }

  /// Get the given key’s corresponding entry in the map for insertion and/or
  /// in-place manipulation.
  ///
  /// Computes in **O(1)** time (amortized average).
  pub fn entry(&mut self, key: impl Into<StaticKey>) -> map::Entry<'_, StaticKey, Value> {
    self.inner.entry(key.into())
  }

  /// Return `true` if an equivalent to `key` exists in the map.
  ///
  /// Computes in **O(1)** time (average).
  pub fn contains_key<Q>(&self, key: &Q) -> bool
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.inner.contains_key(key)
  }

  /// Return a reference to the value stored for `key`, if it is present,
  /// else `None`.
  ///
  /// Computes in **O(1)** time (average).
  pub fn get<Q>(&self, key: &Q) -> Option<&Value>
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.inner.get(key)
  }

  pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.inner.remove(key)
  }

  /// Return references to the key-value pair stored for `key`,
  /// if it is present, else `None`.
  ///
  /// Computes in **O(1)** time (average).
  pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&Key, &Value)>
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.inner.get_key_value(key)
  }

  pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.inner.get_mut(key)
  }

  /// Remove the last key-value pair
  ///
  /// This preserves the order of the remaining elements.
  ///
  /// Computes in **O(1)** time (average).
  pub fn pop(&mut self) -> Option<(StaticKey, Value)> {
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
    F: FnMut(&StaticKey, &mut Value) -> bool,
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
    F: FnMut(&StaticKey, &Value, &StaticKey, &Value) -> Ordering,
  {
    self.inner.sort_by(cmp)
  }

  /// Sort the key-value pairs of the map and return a by-value iterator of
  /// the key-value pairs with the result.
  ///
  /// The sort is stable.
  pub fn sorted_by<F>(self, cmp: F) -> map::IntoIter<StaticKey, Value>
  where
    F: FnMut(&StaticKey, &Value, &StaticKey, &Value) -> Ordering,
  {
    self.inner.sorted_by(cmp)
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
    F: FnMut(&StaticKey, &Value, &StaticKey, &Value) -> Ordering,
  {
    self.inner.sort_unstable_by(cmp)
  }

  /// Sort the key-value pairs of the map and return a by-value iterator of
  /// the key-value pairs with the result.
  ///
  /// The sort is unstable.
  #[inline]
  pub fn sorted_unstable_by<F>(self, cmp: F) -> map::IntoIter<StaticKey, Value>
  where
    F: FnMut(&StaticKey, &Value, &StaticKey, &Value) -> Ordering,
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
pub enum Key<'a> {
  Int(i32),
  Str(Handle<Str>),
  Ref(&'a str),
}

pub type StaticKey = Key<'static>;

impl<'a> Key<'a> {
  pub fn as_str(&self) -> Option<&str> {
    match &self {
      Key::Str(v) => Some(v.as_str()),
      Key::Ref(v) => Some(v),
      Key::Int(_) => None,
    }
  }

  pub(crate) fn write_to_string(&self, s: &mut String) {
    use std::fmt::Write;
    match &self {
      Key::Int(v) => write!(s, "{v}").unwrap(),
      Key::Str(v) => write!(s, "{v}").unwrap(),
      Key::Ref(v) => write!(s, "{v}").unwrap(),
    }
  }

  pub fn to_static(self) -> Key<'static> {
    match self {
      Key::Int(v) => Key::Int(v),
      Key::Str(v) => Key::Str(v),
      Key::Ref(v) => Key::Str(Handle::new(v)),
    }
  }
}

impl From<i32> for Key<'static> {
  fn from(value: i32) -> Self {
    Key::Int(value)
  }
}

impl<'a> From<&'a str> for Key<'a> {
  fn from(value: &'a str) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key::Str(unsafe { Handle::from_ptr_unchecked(Ptr::new(Object::str(value))) })
  }
}

impl<'a> From<Cow<'a, str>> for Key<'a> {
  fn from(value: Cow<'a, str>) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key::Str(unsafe { Handle::from_ptr_unchecked(Ptr::new(Object::str(value.to_string()))) })
  }
}

impl From<Str> for Key<'static> {
  fn from(value: Str) -> Self {
    // SAFETY: The object is guaranteed to be a String
    Key::Str(unsafe { Handle::from_ptr_unchecked(Ptr::new(Object::str(value))) })
  }
}

impl TryFrom<Value> for Key<'static> {
  type Error = InvalidKeyType;

  fn try_from(value: Value) -> Result<Self, Self::Error> {
    if let Some(v) = value.as_int() {
      return Ok(Key::Int(v));
    }
    if let Some(v) = Handle::from_value(value) {
      return Ok(Key::Str(v));
    }
    Err(InvalidKeyType)
  }
}

impl<'a> Equivalent<Key<'a>> for str {
  fn equivalent(&self, key: &Key) -> bool {
    match key {
      Key::Int(_) => false,
      Key::Str(v) => v.as_str() == self,
      Key::Ref(v) => *v == self,
    }
  }
}

impl<'a> Equivalent<Key<'a>> for i32 {
  fn equivalent(&self, key: &Key<'a>) -> bool {
    match key {
      Key::Int(v) => self == v,
      Key::Str(_) => false,
      Key::Ref(_) => false,
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

impl<'a> PartialEq for Key<'a> {
  fn eq(&self, other: &Self) -> bool {
    match (&self, &other) {
      (Key::Int(a), Key::Int(b)) => a == b,
      (Key::Str(a), Key::Str(b)) => a.as_str() == b.as_str(),
      (Key::Ref(a), Key::Ref(b)) => a == b,
      _ => false,
    }
  }
}

impl<'a> Eq for Key<'a> {}

impl<'a> PartialOrd for Key<'a> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    match (&self, &other) {
      (Key::Int(a), Key::Int(b)) => a.partial_cmp(b),
      (Key::Int(_), Key::Str(_)) => Some(std::cmp::Ordering::Less),
      (Key::Str(_), Key::Int(_)) => Some(std::cmp::Ordering::Greater),
      (Key::Str(a), Key::Str(b)) => a.as_str().partial_cmp(b.as_str()),
      (Key::Ref(a), Key::Str(b)) => a.partial_cmp(&b.as_str()),
      (Key::Ref(a), Key::Ref(b)) => a.partial_cmp(b),
      (Key::Str(a), Key::Ref(b)) => a.as_str().partial_cmp(*b),
      (Key::Int(_), Key::Ref(_)) => Some(std::cmp::Ordering::Less),
      (Key::Ref(_), Key::Int(_)) => Some(std::cmp::Ordering::Greater),
    }
  }
}

impl<'a> Ord for Key<'a> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    unsafe { self.partial_cmp(other).unwrap_unchecked() }
  }
}

impl<'a> Hash for Key<'a> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match &self {
      Key::Int(v) => v.hash(state),
      Key::Str(v) => v.hash(state),
      Key::Ref(v) => v.hash(state),
    }
  }
}

impl<'a> std::fmt::Debug for Key<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self {
      Key::Int(v) => std::fmt::Debug::fmt(v, f),
      Key::Str(v) => std::fmt::Debug::fmt(v, f),
      Key::Ref(v) => std::fmt::Debug::fmt(v, f),
    }
  }
}

impl<'a> std::fmt::Display for Key<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self {
      Key::Int(v) => std::fmt::Display::fmt(v, f),
      Key::Str(v) => std::fmt::Display::fmt(v, f),
      Key::Ref(v) => std::fmt::Display::fmt(v, f),
    }
  }
}

impl std::fmt::Debug for Dict {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(&self.inner, f)
  }
}

impl<'a> IntoIterator for &'a Dict {
  type Item = (&'a StaticKey, &'a Value);
  type IntoIter = map::Iter<'a, StaticKey, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.inner.iter()
  }
}

impl<'a> IntoIterator for &'a mut Dict {
  type Item = (&'a StaticKey, &'a mut Value);
  type IntoIter = map::IterMut<'a, StaticKey, Value>;
  fn into_iter(self) -> Self::IntoIter {
    self.inner.iter_mut()
  }
}

impl IntoIterator for Dict {
  type Item = (StaticKey, Value);

  type IntoIter = map::IntoIter<StaticKey, Value>;

  fn into_iter(self) -> Self::IntoIter {
    self.inner.into_iter()
  }
}

impl<Q> Index<&Q> for Dict
where
  Q: ?Sized + Hash + Equivalent<StaticKey>,
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
  Q: ?Sized + Hash + Equivalent<StaticKey>,
{
  /// Returns a mutable reference to the value corresponding to the supplied
  /// `key`.
  ///
  /// ***Panics*** if `key` is not present in the map.
  fn index_mut(&mut self, key: &Q) -> &mut Value {
    self.inner.index_mut(key)
  }
}

impl FromIterator<(StaticKey, Value)> for Dict {
  fn from_iter<T: IntoIterator<Item = (StaticKey, Value)>>(iter: T) -> Self {
    Self {
      inner: Inner::from_iter(iter),
    }
  }
}

impl<const N: usize> From<[(StaticKey, Value); N]> for Dict {
  fn from(arr: [(StaticKey, Value); N]) -> Self {
    Self::from_iter(arr)
  }
}

impl Extend<(StaticKey, Value)> for Dict {
  fn extend<T: IntoIterator<Item = (StaticKey, Value)>>(&mut self, iter: T) {
    self.inner.extend(iter)
  }
}
