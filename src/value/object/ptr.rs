use std::alloc::Layout;
use std::any::TypeId;
use std::cell::Cell;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::{self, NonNull, Pointee};
use std::{alloc, mem};

use crate::ctx::Context;
use crate::error::Result;
use crate::value::object::Object;
use crate::value::Value;

type VTable = <dyn Object as Pointee>::Metadata;

// TODO: identity eq specialization similar to `std::rc::Rc`

#[repr(C)]
struct Repr<T> {
  // TODO: can we get rid of layout here?
  layout: Layout,
  type_id: TypeId,
  refs: Cell<u64>,
  vtable: VTable,
  data: T,
}

pub struct Ptr<T> {
  repr: NonNull<Repr<T>>,
}

impl<T> Ptr<T> {
  fn inner(&self) -> &Repr<T> {
    unsafe { self.repr.as_ref() }
  }

  pub fn into_ref<'cx>(self) -> Ref<'cx, T> {
    Ref {
      ptr: self,
      lifetime: PhantomData,
    }
  }

  pub(crate) fn refs(&self) -> u64 {
    self.inner().refs.get()
  }

  pub(crate) fn into_addr(self) -> usize {
    let ptr = self.repr.as_ptr();
    mem::forget(self);

    ptr as usize
  }

  pub(crate) unsafe fn from_addr(addr: usize) -> Self {
    let ptr = addr as *mut Repr<T>;

    Self {
      repr: NonNull::new_unchecked(ptr),
    }
  }

  pub(crate) unsafe fn incref_addr(addr: usize) {
    let ptr = Self::from_addr(addr);
    Self::incref(ptr.repr);
    mem::forget(ptr);
  }

  unsafe fn incref(ptr: NonNull<Repr<T>>) {
    let repr = unsafe { ptr.as_ref() };
    repr.refs.set(repr.refs.get() + 1);
  }

  /* pub(crate) unsafe fn decref_addr(addr: usize) {
    let ptr = unsafe { NonNull::new_unchecked(addr as *mut Repr<T>) };
    Self::decref(ptr)
  } */

  unsafe fn decref(ptr: NonNull<Repr<T>>) {
    let repr = unsafe { ptr.as_ref() };
    repr.refs.set(repr.refs.get() - 1);
  }

  pub fn ptr_hash<H: Hasher>(&self, state: &mut H) {
    self.repr.hash(state)
  }

  pub fn ptr_eq(&self, other: &Ptr<T>) -> bool {
    self.repr.eq(&other.repr)
  }
}

impl<T> Deref for Ptr<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner().data
  }
}

impl<T> Drop for Ptr<T> {
  fn drop(&mut self) {
    if self.refs() > 1 {
      unsafe { Self::decref(self.repr) };
    } else {
      unsafe { ptr::drop_in_place(&mut self.repr.as_mut().data as *mut _) };

      let ptr = self.repr.as_ptr() as *mut u8;
      let layout = self.inner().layout;
      // TODO: replace with `alloc::Global.deallocate` when `alloc::Global` is stable
      unsafe { alloc::dealloc(ptr, layout) }
    }
  }
}

impl<T> Clone for Ptr<T> {
  fn clone(&self) -> Self {
    unsafe { Self::incref(self.repr) };
    Self { repr: self.repr }
  }
}

impl<T: Object> Object for Ptr<T> {
  fn type_name(&self) -> &'static str {
    self.inner().data.type_name()
  }

  fn get_field(&self, cx: &Context, key: &str) -> Result<Option<Value>> {
    self.inner().data.get_field(cx, key)
  }

  fn set_field(&self, cx: &Context, key: &str, value: Value) -> Result<()> {
    self.inner().data.set_field(cx, key, value)
  }
}

impl<T: Debug> Debug for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.inner().data, f)
  }
}

impl<T: Display> Display for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.inner().data, f)
  }
}

impl<T: PartialEq> PartialEq for Ptr<T> {
  fn eq(&self, other: &Self) -> bool {
    self.inner().data == other.inner().data
  }
}

impl<T: Eq> Eq for Ptr<T> {}

impl<T: PartialOrd> PartialOrd for Ptr<T> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.inner().data.partial_cmp(&other.inner().data)
  }
}

impl<T: Ord> Ord for Ptr<T> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.inner().data.cmp(&other.inner().data)
  }
}

impl<T: Hash> Hash for Ptr<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inner().data.hash(state);
  }
}

impl<T> std::borrow::Borrow<T> for Ptr<T> {
  fn borrow(&self) -> &T {
    self
  }
}

impl<T> AsRef<T> for Ptr<T> {
  fn as_ref(&self) -> &T {
    self
  }
}

impl Context {
  pub fn alloc<T: Object + 'static>(&self, v: T) -> Ptr<T> {
    let object = Box::new(Repr {
      layout: Layout::new::<Repr<T>>(),
      type_id: TypeId::of::<T>(),
      refs: Cell::new(1),
      vtable: ptr::metadata(&v as &dyn Object),
      data: v,
    });
    Ptr {
      repr: unsafe { NonNull::new_unchecked(Box::into_raw(object)) },
    }
  }
}

pub struct Ref<'cx, T> {
  ptr: Ptr<T>,
  lifetime: PhantomData<&'cx T>,
}

impl<'cx, T> Deref for Ref<'cx, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.ptr.deref()
  }
}

/// Calculates the offset of the specified field from the start of the named
/// struct.
macro_rules! offset_of {
  ($ty: path, $field: tt) => {{
    // ensure the type is a named struct + field exists and is accessible
    let $ty { $field: _, .. };
    let uninit = <::core::mem::MaybeUninit<$ty>>::uninit();
    let base_ptr: *const $ty = uninit.as_ptr();
    #[allow(unused_unsafe)]
    let field_ptr = unsafe { ::core::ptr::addr_of!((*base_ptr).$field) };
    (field_ptr as usize) - (base_ptr as usize)
  }};
}

pub struct Any {
  _private: (),
}

impl Any {
  unsafe fn get_repr_ptr(&self) -> *const Repr<()> {
    let data_offset = offset_of!(Repr<()>, data);
    let ptr = self as *const Any as *const u8;
    ptr.sub(data_offset) as *const Repr<()>
  }

  unsafe fn as_dyn_object_ptr(&self) -> *const dyn Object {
    let ptr = self.get_repr_ptr();
    ptr::from_raw_parts::<dyn Object>(ptr::addr_of!((*ptr).data), (*ptr).vtable)
  }

  unsafe fn as_dyn_object(&self) -> &dyn Object {
    &*self.as_dyn_object_ptr()
  }
}

impl Drop for Any {
  fn drop(&mut self) {
    unsafe { ptr::drop_in_place(self.as_dyn_object_ptr() as *mut dyn Object) }
  }
}

impl Object for Any {
  fn type_name(&self) -> &'static str {
    let this = unsafe { self.as_dyn_object() };
    this.type_name()
  }

  fn get_field(&self, cx: &Context, key: &str) -> Result<Option<Value>> {
    let this = unsafe { self.as_dyn_object() };
    this.get_field(cx, key)
  }

  fn set_field(&self, cx: &Context, key: &str, value: Value) -> Result<()> {
    let this = unsafe { self.as_dyn_object() };
    this.set_field(cx, key, value)
  }
}

impl Debug for Any {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let this = unsafe { self.as_dyn_object() };
    Debug::fmt(this, f)
  }
}

impl Display for Any {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let this = unsafe { self.as_dyn_object() };
    Display::fmt(this, f)
  }
}

impl<T: Object> Ptr<T> {
  pub fn into_any(self) -> Ptr<Any> {
    unsafe { mem::transmute::<Ptr<T>, Ptr<Any>>(self) }
  }
}

impl Ptr<Any> {
  pub fn cast<T: Object>(self) -> Result<Ptr<T>, Ptr<Any>> {
    match self.inner().type_id == TypeId::of::<T>() {
      true => Ok(unsafe { mem::transmute::<Ptr<Any>, Ptr<T>>(self) }),
      false => Err(self),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::rc::Rc;

  use super::*;

  struct Foo {
    value: u64,
    on_drop: Box<dyn FnMut()>,
  }

  impl Object for Foo {
    fn type_name(&self) -> &'static str {
      "Foo"
    }
  }

  impl Debug for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("Foo").field("value", &self.value).finish()
    }
  }

  impl Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      Debug::fmt(self, f)
    }
  }

  impl Drop for Foo {
    fn drop(&mut self) {
      (self.on_drop)();
    }
  }

  #[derive(Debug)]
  struct Bar {
    // it's not dead, we're using it via the `Debug` impl
    #[allow(dead_code)]
    value: u64,
  }

  impl Object for Bar {
    fn type_name(&self) -> &'static str {
      "Bar"
    }
  }

  impl Display for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      Debug::fmt(self, f)
    }
  }

  fn noop() {}

  #[allow(clippy::redundant_clone)]
  #[test]
  fn object_repr_refcount() {
    let cx = Context::for_test();

    let foo = cx.alloc(Foo {
      value: 100,
      on_drop: Box::new(noop),
    });
    assert_eq!(foo.refs(), 1);
    let foo2 = foo.clone();
    assert_eq!(foo.refs(), 2);
    drop(foo2);
    assert_eq!(foo.refs(), 1);
    drop(foo);
  }

  #[test]
  fn object_any_refcount() {
    let cx = Context::for_test();

    let foo = cx
      .alloc(Foo {
        value: 100,
        on_drop: Box::new(noop),
      })
      .into_any();
    assert_eq!(foo.refs(), 1);
    let foo2 = foo.clone();
    assert_eq!(foo.refs(), 2);
    drop(foo2);
    assert_eq!(foo.refs(), 1);
    drop(foo);
  }

  #[test]
  fn ptr_dyn_cast() {
    let cx = Context::for_test();

    let foo = cx.alloc(Foo {
      value: 100,
      on_drop: Box::new(noop),
    });
    let foo = foo.into_any();
    assert_eq!(foo.type_name(), "Foo");
    let foo = foo.cast::<Foo>().unwrap();
    assert_eq!(foo.value, 100);
    drop(foo);
  }

  #[test]
  fn drop_is_called() {
    let cx = Context::for_test();

    // static
    {
      let dropped = Rc::new(RefCell::new(false));
      let foo = cx.alloc(Foo {
        value: 100,
        on_drop: Box::new({
          let dropped = dropped.clone();
          move || *dropped.borrow_mut() = true
        }),
      });
      drop(foo);
      assert!(*dropped.borrow());
    }

    // dynamic
    {
      let dropped = Rc::new(RefCell::new(false));
      let foo = cx.alloc(Foo {
        value: 100,
        on_drop: Box::new({
          let dropped = dropped.clone();
          move || *dropped.borrow_mut() = true
        }),
      });
      let foo = foo.into_any();
      drop(foo);
      assert!(*dropped.borrow());
    }
  }

  #[test]
  fn any_casting() {
    let cx = Context::for_test();

    let v = cx.alloc(Bar { value: 100 });
    let v = v.into_any();
    let v = v.cast::<Foo>().unwrap_err();
    let _ = v.cast::<Bar>().unwrap();
  }

  #[test]
  fn debug_and_display_fmt() {
    let cx = Context::for_test();

    let v = cx.alloc(Bar { value: 100 });
    assert_eq!("Bar { value: 100 }", v.to_string());
    let v = v.into_any();
    assert_eq!("Bar { value: 100 }", v.to_string());
  }
}
