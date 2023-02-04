use std::marker::PhantomData;

use super::ObjectHandle;
use crate::ptr::{Ref, RefMut};
use crate::{Object, Ptr, Value};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Handle<T> {
  o: Ptr<Object>,
  _p: PhantomData<T>,
}

impl<T: ObjectHandle> Handle<T> {
  pub fn from_ptr(o: Ptr<Object>) -> Option<Self> {
    if !<T as ObjectHandle>::is_self(&o) {
      return None;
    }
    Some(Self { o, _p: PhantomData })
  }

  /// ### Safety
  /// `o` must be an instance of `T`
  pub unsafe fn from_ptr_unchecked(o: Ptr<Object>) -> Self {
    debug_assert!(<T as ObjectHandle>::is_self(&o));
    Self { o, _p: PhantomData }
  }

  pub fn from_value(v: Value) -> Option<Self> {
    v.into_object().and_then(Handle::from_ptr)
  }

  pub fn borrow(&self) -> Ref<'_, T> {
    // SAFETY: Valid by construction in `new`
    unsafe { <T as ObjectHandle>::as_self(&self.o).unwrap_unchecked() }
  }

  pub fn borrow_mut(&mut self) -> RefMut<'_, T> {
    // SAFETY: Valid by construction in `new`
    unsafe { <T as ObjectHandle>::as_self_mut(&mut self.o).unwrap_unchecked() }
  }

  /// Widen the type back to `Object`
  pub fn widen(self) -> Ptr<Object> {
    self.o
  }
}

impl<T: ObjectHandle> std::fmt::Debug for Handle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(&self.o, f)
  }
}

impl<T: ObjectHandle> std::fmt::Display for Handle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.o, f)
  }
}
