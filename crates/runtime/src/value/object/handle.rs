use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use super::{Object, ObjectHandle, Ptr, Value};

#[derive(Clone)]
pub struct Handle<T> {
  o: Ptr<Object>,
  _p: PhantomData<T>,
}

impl<T: ObjectHandle> Handle<T> {
  pub fn new(o: impl Into<Object>) -> Self {
    unsafe { Self::from_ptr_unchecked(Ptr::new(o.into())) }
  }

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

  /// Widen the type back to `Object`
  pub fn widen(self) -> Ptr<Object> {
    self.o
  }

  pub fn strong_count(&self) -> usize {
    Ptr::strong_count(&self.o)
  }
}

impl<T: ObjectHandle> Deref for Handle<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    // SAFETY: Valid by construction in `new`
    unsafe { <T as ObjectHandle>::as_self(&self.o).unwrap_unchecked() }
  }
}

impl<T: ObjectHandle> DerefMut for Handle<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: Valid by construction in `new`
    unsafe { <T as ObjectHandle>::as_self_mut(&mut self.o).unwrap_unchecked() }
  }
}

impl<T: ObjectHandle + Into<Object>> From<T> for Handle<T> {
  fn from(value: T) -> Self {
    let obj = Ptr::new(value.into());
    unsafe { Handle::from_ptr_unchecked(obj) }
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
