use core::cell::UnsafeCell;
use core::fmt::{Debug, Display};
use core::mem::transmute;
use core::ops::{Deref, Index};
use core::slice::SliceIndex;

use allocator_api2::vec::Vec;
use bumpalo::AllocErr;

use crate::gc::{Alloc, Gc, NoAlloc, Object, Ref, NO_ALLOC};
use crate::util::DelegateDebugToDisplay;
use crate::val::Value;

pub struct List {
  vec: UnsafeCell<Vec<Value, NoAlloc>>,
}

impl List {
  pub fn try_new_in(gc: &Gc) -> Result<Ref<Self>, AllocErr> {
    gc.try_alloc(List {
      vec: UnsafeCell::new(Vec::new_in(NO_ALLOC)),
    })
  }

  pub fn try_with_capacity_in(gc: &Gc, capacity: usize) -> Result<Ref<Self>, AllocErr> {
    let mut vec = Vec::<Value, _>::new_in(Alloc::new(gc));
    vec.try_reserve_exact(capacity).map_err(|_| AllocErr)?;
    let vec = unsafe { transmute::<_, Vec<Value, NoAlloc>>(vec) };
    gc.try_alloc(List {
      vec: UnsafeCell::new(vec),
    })
  }

  #[inline]
  pub fn len(&self) -> usize {
    self.get_vec().len()
  }

  #[inline]
  pub fn capacity(&self) -> usize {
    self.get_vec().capacity()
  }

  #[inline]
  pub fn is_empty(&self) -> bool {
    self.get_vec().is_empty()
  }

  #[inline]
  pub fn try_push(&self, gc: &Gc, value: Value) -> Result<(), AllocErr> {
    let vec = self.get_vec_mut_alloc(gc);
    vec.try_reserve(1).map_err(|_| AllocErr)?;
    unsafe { self.try_push_no_grow(value).unwrap_unchecked() }
    Ok(())
  }

  #[inline]
  pub fn try_push_no_grow(&self, value: Value) -> Result<(), Value> {
    self.get_vec_mut_no_alloc().push_within_capacity(value)
  }

  #[inline]
  pub fn pop(&self) -> Option<Value> {
    self.get_vec_mut_no_alloc().pop()
  }

  #[inline]
  pub fn as_slice(&self) -> &[Value] {
    self.get_vec().as_slice()
  }

  #[inline]
  fn get_vec(&self) -> &Vec<Value, NoAlloc> {
    unsafe { self.vec.get().as_ref().unwrap_unchecked() }
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  fn get_vec_mut_no_alloc(&self) -> &mut Vec<Value, NoAlloc> {
    unsafe { self.vec.get().as_mut().unwrap_unchecked() }
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  fn get_vec_mut_alloc<'gc>(&self, gc: &'gc Gc) -> &mut Vec<Value, Alloc<'gc>> {
    let vec = unsafe { self.vec.get().as_mut().unwrap_unchecked() };
    let vec = unsafe { transmute::<_, &mut Vec<Value, Alloc<'gc>>>(vec) };
    vec.allocator().set(gc);
    vec
  }
}

impl Object for List {
  // We don't want to call `Drop` on the contents of the inner `Vec`.
  // The `List` object and its backing storage will be deallocated
  // by the GC at some point.
  const NEEDS_DROP: bool = false;
}

impl Debug for List {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Debug::fmt(self.as_slice(), f)
  }
}

impl Display for List {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut f = f.debug_list();
    for entry in self.get_vec().iter() {
      f.entry(&DelegateDebugToDisplay(entry));
    }
    f.finish()
  }
}

impl Deref for List {
  type Target = [Value];

  fn deref(&self) -> &Self::Target {
    self.as_slice()
  }
}

impl<Idx: SliceIndex<[Value]>> Index<Idx> for List {
  type Output = Idx::Output;

  fn index(&self, index: Idx) -> &Self::Output {
    self.as_slice().index(index)
  }
}

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
    assert_eq!(list[0].cast::<i32>().unwrap(), 10i32);
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
    assert_eq!(list[0].cast::<i32>().unwrap(), 10i32);
    let value = list.pop().unwrap();
    assert_eq!(list.len(), 0);
    assert!(list.capacity() > 0);
    assert_eq!(value.cast::<i32>().unwrap(), 10i32);
  }
}
