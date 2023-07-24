use core::cell::UnsafeCell;
use core::fmt::{Debug, Display};

use crate::ds::vec::{GcVec, GcVecN};
use crate::ds::{HasAlloc, HasNoAlloc};
use crate::error::AllocError;
use crate::gc::{Alloc, Gc, Object, Ref, NO_ALLOC};
use crate::op::Reg;
use crate::util::DelegateDebugToDisplay;
use crate::val::Value;

pub struct List {
  vec: UnsafeCell<GcVecN<Value>>,
}

impl List {
  pub fn try_new_in(gc: &Gc) -> Result<Ref<Self>, AllocError> {
    gc.try_alloc(List {
      vec: UnsafeCell::new(GcVecN::new_in(NO_ALLOC)),
    })
  }

  pub fn try_with_capacity_in(gc: &Gc, capacity: usize) -> Result<Ref<Self>, AllocError> {
    let mut vec = GcVec::new_in(Alloc::new(gc));
    vec.try_reserve_exact(capacity).map_err(|_| AllocError)?;
    let vec = UnsafeCell::new(vec.to_no_alloc());

    gc.try_alloc(List { vec })
  }

  #[inline]
  pub fn len(&self) -> usize {
    self.vec().len()
  }

  #[inline]
  pub fn capacity(&self) -> usize {
    self.vec().capacity()
  }

  #[inline]
  pub fn is_empty(&self) -> bool {
    self.vec().is_empty()
  }

  #[inline]
  pub fn try_push(&self, gc: &Gc, value: Value) -> Result<(), AllocError> {
    let vec = self.vec_alloc(gc);
    vec.try_reserve(1).map_err(|_| AllocError)?;
    unsafe { self.try_push_no_grow(value).unwrap_unchecked() }
    Ok(())
  }

  #[inline]
  pub fn try_push_no_grow(&self, value: Value) -> Result<(), Value> {
    self.vec_mut().push_within_capacity(value)
  }

  #[inline]
  pub fn pop(&self) -> Option<Value> {
    self.vec_mut().pop()
  }

  #[inline]
  pub fn get(&self, index: usize) -> Option<Value> {
    self.vec().get(index).copied()
  }

  /// # Safety
  /// `index` must be a valid index
  #[inline]
  pub unsafe fn get_unchecked(&self, index: usize) -> Value {
    *self.vec().get_unchecked(index)
  }

  #[inline]
  pub fn set(&self, index: usize, value: Value) -> bool {
    if let Some(slot) = self.vec_mut().get_mut(index) {
      *slot = value;
      true
    } else {
      false
    }
  }

  pub fn extend_from_slice(&self, gc: &Gc, items: &[Value]) -> Result<(), AllocError> {
    let vec = self.vec_alloc(gc);
    vec.try_reserve(items.len()).map_err(|_| AllocError)?;
    let len = vec.len();
    for (i, item) in items.iter().enumerate() {
      unsafe {
        vec.as_mut_ptr().add(i).write(*item);
      }
    }
    unsafe {
      vec.set_len(len + items.len());
    }
    Ok(())
  }

  /// # Safety
  /// `index` must be a valid index
  #[inline]
  pub unsafe fn set_unchecked(&self, index: usize, value: Value) {
    let slot = self.vec_mut().get_unchecked_mut(index);
    *slot = value;
  }

  /// # Safety
  /// `self` must not be mutated for the lifetime of the returned slice,
  /// as this could cause the `Vec` to reallocate its backing storage,
  /// invalidating the slice.
  #[inline]
  pub unsafe fn as_slice(&self) -> &[Value] {
    self.vec().as_slice()
  }

  #[inline]
  fn vec(&self) -> &GcVecN<Value> {
    unsafe { &*self.vec.get() }
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  fn vec_mut(&self) -> &mut GcVecN<Value> {
    unsafe { &mut *self.vec.get() }
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  fn vec_alloc<'gc>(&self, gc: &'gc Gc) -> &mut GcVec<'gc, Value> {
    let vec = unsafe { &mut *self.vec.get() };
    vec.as_alloc_mut(gc)
  }
}

/* #[inline(always)]
fn handle_size_hint(size_hint: (usize, Option<usize>)) -> usize {
  size_hint.1.unwrap_or(size_hint.0)
} */

impl Object for List {
  // We don't want to call `Drop` on the contents of the inner `Vec`.
  // The `List` object and its backing storage will be deallocated
  // by the GC at some point.
  const NEEDS_DROP: bool = false;
}

impl Debug for List {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    // This is fine because it's a short-lived borrow
    Debug::fmt(unsafe { self.as_slice() }, f)
  }
}

impl Display for List {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut f = f.debug_list();
    for entry in self.vec().iter() {
      f.entry(&DelegateDebugToDisplay(entry));
    }
    f.finish()
  }
}

#[derive(Debug)]
pub struct ListDescriptor {
  start: Reg<u8>,
  count: u8,
}

impl ListDescriptor {
  pub fn try_new_in(gc: &Gc, start: Reg<u8>, count: u8) -> Result<Ref<Self>, AllocError> {
    gc.try_alloc(ListDescriptor { start, count })
  }

  #[inline]
  pub fn start(&self) -> Reg<u8> {
    self.start
  }

  #[inline]
  pub fn count(&self) -> u8 {
    self.count
  }
}

impl Display for ListDescriptor {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<list>")
  }
}

impl Object for ListDescriptor {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn list_ops_new() {
    let gc = Gc::new();

    let list = List::try_new_in(&gc).unwrap();
    assert_eq!(list.len(), 0);
    assert_eq!(list.capacity(), 0);

    list.try_push(&gc, Value::new(10i32)).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list.capacity() > 0);
    assert_eq!(list.get(0).unwrap().cast::<i32>().unwrap(), 10i32);

    let value = list.pop().unwrap();
    assert_eq!(list.len(), 0);
    assert!(list.capacity() > 0);
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }

  #[test]
  fn list_ops_with_cap() {
    let gc = Gc::new();

    let list = List::try_with_capacity_in(&gc, 1).unwrap();
    assert_eq!(list.len(), 0);
    assert!(list.capacity() > 0);

    list.try_push(&gc, Value::new(10i32)).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list.capacity() > 0);
    assert_eq!(list.get(0).unwrap().cast::<i32>().unwrap(), 10i32);

    let value = list.pop().unwrap();
    assert_eq!(list.len(), 0);
    assert!(list.capacity() > 0);
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }
}
