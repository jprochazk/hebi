use std::fmt::{Debug, Display};
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

use crate::Value;

#[derive(Clone, Default)]
pub struct List(Vec<Value>);

impl List {
  pub fn iter(&self) -> std::slice::Iter<'_, Value> {
    self.0.iter()
  }

  pub fn new() -> Self {
    Self(Vec::new())
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self(Vec::with_capacity(capacity))
  }

  pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Value> {
    self.0.iter_mut()
  }

  pub fn push(&mut self, value: Value) {
    self.0.push(value)
  }

  pub fn pop(&mut self) -> Option<Value> {
    self.0.pop()
  }

  pub fn extend<I>(&mut self, iter: I)
  where
    I: IntoIterator<Item = Value>,
  {
    self.0.extend(iter)
  }

  pub fn truncate(&mut self, len: usize) {
    self.0.truncate(len)
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }
}

impl From<Vec<Value>> for List {
  fn from(value: Vec<Value>) -> Self {
    Self(value)
  }
}

impl<'a> From<&'a [Value]> for List {
  fn from(value: &'a [Value]) -> Self {
    Self(value.to_vec())
  }
}

impl<const N: usize> From<[Value; N]> for List {
  fn from(value: [Value; N]) -> Self {
    Self(value.to_vec())
  }
}

impl Display for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.0, f)
  }
}

impl Debug for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.0, f)
  }
}

impl<Idx> Index<Idx> for List
where
  Idx: SliceIndex<[Value]>,
{
  type Output = Idx::Output;

  #[inline(always)]
  fn index(&self, index: Idx) -> &Self::Output {
    self.0.index(index)
  }
}

impl<Idx> IndexMut<Idx> for List
where
  Idx: SliceIndex<[Value]>,
{
  #[inline]
  fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
    self.0.index_mut(index)
  }
}
