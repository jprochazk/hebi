use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use super::{Object, ObjectType, Ptr};

#[derive(Clone)]
pub struct Handle<T> {
  obj: Ptr<Object>,
  _p: PhantomData<T>,
}

impl<T: ObjectType> Handle<T> {
  pub fn alloc(obj: T) -> Self {
    unsafe { Self::from_ptr_unchecked(Ptr::alloc(obj.into())) }
  }

  pub(crate) fn from_ptr(obj: Ptr<Object>) -> Option<Self> {
    if !<T as ObjectType>::is(unsafe { obj._get() }) {
      return None;
    }
    Some(Self {
      obj,
      _p: PhantomData,
    })
  }

  /// ### Safety
  /// `o` must be an instance of `T`
  pub(crate) unsafe fn from_ptr_unchecked(obj: Ptr<Object>) -> Self {
    debug_assert!(<T as ObjectType>::is(unsafe { obj._get() }));
    Self {
      obj,
      _p: PhantomData,
    }
  }

  /// Widen the type back to `Object`
  pub(crate) fn widen(self) -> Ptr<Object> {
    self.obj
  }

  /* pub(crate) fn strong_count(&self) -> usize {
    Ptr::strong_count(&self.obj)
  } */

  /// Do not use directly.
  #[doc(hidden)]
  pub(crate) unsafe fn _get(&self) -> &T {
    let obj = unsafe { self.obj._get() };
    debug_assert!(<T as ObjectType>::is(obj));
    unsafe { T::as_ref(obj).unwrap_unchecked() }
  }

  /// Do not use directly.
  #[doc(hidden)]
  pub(crate) unsafe fn _get_mut(&mut self) -> &mut T {
    let obj = unsafe { self.obj._get_mut() };
    debug_assert!(<T as ObjectType>::is(obj));
    unsafe { T::as_mut(obj).unwrap_unchecked() }
  }
}

impl<T: ObjectType + Display> Display for Handle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(unsafe { self._get() }, f)
  }
}

impl<T: ObjectType + Hash> Hash for Handle<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    unsafe { self._get() }.hash(state)
  }
}

impl<T: ObjectType + PartialEq> PartialEq for Handle<T> {
  fn eq(&self, other: &Self) -> bool {
    unsafe { self._get() == other._get() }
  }
}

impl<T: ObjectType + Eq> Eq for Handle<T> {}
