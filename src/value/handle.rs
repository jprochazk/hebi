use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use super::object::{Access, Object, ObjectType, Str};
use super::ptr::Ptr;
use super::Value;
use crate::ctx::Context;

pub struct Ref<'vm, T> {
  handle: Handle<T>,
  _p: PhantomData<&'vm T>,
}

impl<'vm, T: ObjectType> Deref for Ref<'vm, T> {
  type Target = Handle<T>;

  fn deref(&self) -> &Self::Target {
    &self.handle
  }
}

impl<'vm, T: ObjectType> DerefMut for Ref<'vm, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.handle
  }
}

pub struct Handle<T> {
  obj: Ptr<Object>,
  _p: PhantomData<T>,
}

impl<T: ObjectType> Handle<T> {
  pub fn _alloc(obj: T) -> Self {
    unsafe { Self::from_ptr_unchecked(Ptr::alloc(obj.into())) }
  }

  pub fn from_ptr(obj: Ptr<Object>) -> Option<Self> {
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
  pub unsafe fn from_ptr_unchecked(obj: Ptr<Object>) -> Self {
    debug_assert!(<T as ObjectType>::is(unsafe { obj._get() }));
    Self {
      obj,
      _p: PhantomData,
    }
  }

  /// Widen the type back to `Object`
  pub fn widen(self) -> Ptr<Object> {
    self.obj
  }

  /* pub fn strong_count(&self) -> usize {
    Ptr::strong_count(&self.obj)
  } */

  /// Do not use directly.
  #[doc(hidden)]
  pub unsafe fn _get(&self) -> &T {
    let obj = unsafe { self.obj._get() };
    debug_assert!(<T as ObjectType>::is(obj));
    unsafe { T::as_ref(obj).unwrap_unchecked() }
  }

  /// Do not use directly.
  #[doc(hidden)]
  pub unsafe fn _get_mut(&mut self) -> &mut T {
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

impl<T: ObjectType + PartialOrd> PartialOrd for Handle<T> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    unsafe { self._get() }.partial_cmp(unsafe { other._get() })
  }
}

impl<T: ObjectType + Ord> Ord for Handle<T> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    unsafe { self._get() }.cmp(unsafe { other._get() })
  }
}

impl<T: ObjectType> Access for Handle<T> {
  fn is_frozen(&self) -> bool {
    unsafe { self._get() }.is_frozen()
  }

  fn should_bind_methods(&self) -> bool {
    unsafe { self._get() }.should_bind_methods()
  }

  fn field_get(&self, ctx: &Context, key: &str) -> crate::Result<Option<Value>> {
    unsafe { self._get() }.field_get(ctx, key)
  }

  fn field_set(&mut self, ctx: &Context, key: Handle<Str>, value: Value) -> crate::Result<()> {
    unsafe { self._get_mut() }.field_set(ctx, key, value)
  }

  fn index_get(&self, ctx: &Context, key: Value) -> crate::Result<Option<Value>> {
    unsafe { self._get() }.index_get(ctx, key)
  }

  fn index_set(&mut self, ctx: &Context, key: Value, value: Value) -> crate::Result<()> {
    unsafe { self._get_mut() }.index_set(ctx, key, value)
  }
}

impl<T> Clone for Handle<T> {
  fn clone(&self) -> Self {
    Self {
      obj: self.obj.clone(),
      _p: self._p,
    }
  }
}
