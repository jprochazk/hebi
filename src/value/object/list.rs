use std::fmt::Display;
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

use super::{Access, Handle};
use crate::value::Value;
use crate::RuntimeError;

#[derive(Clone, Default)]
pub struct List(Vec<Value>);

impl List {
  pub fn new() -> Self {
    Self(Vec::new())
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self(Vec::with_capacity(capacity))
  }
}

#[derive::delegate_to_handle]
impl List {
  pub fn iter(&self) -> std::slice::Iter<'_, Value> {
    self.0.iter()
  }

  pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Value> {
    self.0.iter_mut()
  }

  pub fn get(&self, index: usize) -> Option<Value> {
    self.0.get(index).cloned()
  }

  pub fn set(&mut self, index: usize, value: Value) {
    if let Some(slot) = self.0.get_mut(index) {
      *slot = value;
    }
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

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
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

impl<Idx> Index<Idx> for Handle<List>
where
  Idx: SliceIndex<[Value]>,
{
  type Output = Idx::Output;

  #[inline(always)]
  fn index(&self, index: Idx) -> &Self::Output {
    unsafe { self._get() }.index(index)
  }
}

impl<Idx> IndexMut<Idx> for Handle<List>
where
  Idx: SliceIndex<[Value]>,
{
  #[inline]
  fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
    unsafe { self._get_mut() }.index_mut(index)
  }
}

impl Access for List {
  fn is_frozen(&self) -> bool {
    false
  }

  // TODO: tests
  fn field_get(&self, key: &str) -> Result<Option<Value>, crate::RuntimeError> {
    // TODO: methods (push, pop, etc.)
    Ok(match key {
      "len" => Some(Value::int(self.0.len() as i32)),
      _ => None,
    })
  }

  fn index_get(&self, key: Value) -> Result<Option<Value>, crate::RuntimeError> {
    let Some(index) = key.clone().to_int() else {
      return Err(crate::RuntimeError::script(format!("cannot index list using {key}"), 0..0));
    };
    Ok(calculate_index(index, self.len()).and_then(|index| self.0.get(index).cloned()))
  }

  fn index_set(&mut self, key: Value, value: Value) -> Result<(), crate::RuntimeError> {
    let Some(index) = key.clone().to_int() else {
      return Err(crate::RuntimeError::script(format!("cannot index list using {key}"), 0..0));
    };
    let index = calculate_index(index, self.len())
      .ok_or_else(|| RuntimeError::script(format!("index {index} is out of bounds"), 0..0))?;
    let Some(slot) = self.0.get_mut(index) else {
      return Err(RuntimeError::script(format!("index {index} is out of bounds"), 0..0));
    };
    *slot = value;
    Ok(())
  }
}

fn calculate_index(index: i32, len: usize) -> Option<usize> {
  let (index, len) = (index as isize, len as isize);
  if index >= 0 {
    return Some(index as usize);
  }
  if index < 0 && (-index) > len {
    return Some((len - index) as usize);
  }
  None
}
fn handle_negative_index(index: isize, len: isize) -> Option<usize> {
  if (-index) > len {
    None
  } else {
    Some((len - index) as usize)
  }
}

impl Display for List {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "[")?;
    let mut iter = self.0.iter().peekable();
    while let Some(item) = iter.next() {
      write!(f, "{item}")?;
      if iter.peek().is_some() {
        write!(f, ", ")?;
      }
    }
    write!(f, "]")
  }
}
