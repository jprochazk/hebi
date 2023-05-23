use std::alloc::Layout;
use std::any::TypeId;
use std::cell::Cell;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::ptr::{self, NonNull};
use std::{alloc, mem};

use super::{Type, VTable};
use crate::vm::global::Global;
use crate::Result;

// TODO: identity eq specialization similar to `std::rc::Rc`

#[repr(C)]
struct Repr<T: Sized + 'static> {
  layout: Layout,
  type_id: TypeId,
  refs: Cell<u64>,
  vtable: &'static super::VTable<T>,
  data: T,
}

pub struct Ptr<T: Sized + 'static> {
  repr: NonNull<Repr<T>>,
}

impl<T: Sized + 'static> Ptr<T> {
  fn repr(&self) -> &Repr<T> {
    unsafe { self.repr.as_ref() }
  }

  pub(crate) fn refs(&self) -> u64 {
    self.repr().refs.get()
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

  pub fn ty(&self) -> TypeId {
    self.repr().type_id
  }
}

impl<T: Sized + 'static> Deref for Ptr<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.repr().data
  }
}

impl<T: Sized + 'static> Drop for Ptr<T> {
  fn drop(&mut self) {
    if self.refs() > 1 {
      unsafe { Self::decref(self.repr) };
    } else {
      unsafe { ptr::drop_in_place((&mut self.repr.as_mut().data) as *mut _) };

      let ptr = self.repr.as_ptr() as *mut u8;
      let layout = self.repr().layout;
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

impl<T: Debug> Debug for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.repr().data, f)
  }
}

impl<T: Display> Display for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.repr().data, f)
  }
}

impl<T: PartialEq> PartialEq for Ptr<T> {
  fn eq(&self, other: &Self) -> bool {
    self.repr().data == other.repr().data
  }
}

impl<T: Eq> Eq for Ptr<T> {}

impl<T: PartialOrd> PartialOrd for Ptr<T> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.repr().data.partial_cmp(&other.repr().data)
  }
}

impl<T: Ord> Ord for Ptr<T> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.repr().data.cmp(&other.repr().data)
  }
}

impl<T: Hash> Hash for Ptr<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.repr().data.hash(state);
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

impl<T: Type + Sized + 'static> Ptr<T> {
  pub(crate) unsafe fn alloc_raw(v: T) -> Self {
    let object = Box::new(Repr {
      layout: Layout::new::<Repr<T>>(),
      type_id: TypeId::of::<T>(),
      refs: Cell::new(1),
      vtable: <T as Type>::vtable(),
      data: v,
    });
    Ptr {
      repr: NonNull::new_unchecked(Box::into_raw(object)),
    }
  }
}

impl Global {
  pub fn alloc<T: Type + 'static>(&self, v: T) -> Ptr<T> {
    unsafe { Ptr::alloc_raw(v) }
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
  __: (),
}

impl Any {
  pub(crate) unsafe fn vtable(&self) -> &'static VTable<()> {
    std::ptr::read(std::ptr::addr_of!((*self.repr_raw()).vtable))
  }

  unsafe fn repr_raw(&self) -> *const Repr<()> {
    let data_offset = offset_of!(Repr<()>, data);
    let ptr = self as *const Any as *const u8;
    ptr.sub(data_offset) as *const Repr<()>
  }
}

impl Drop for Any {
  fn drop(&mut self) {
    unsafe {
      let drop_in_place = self.vtable().drop_in_place;
      drop_in_place(self as *mut Any as *mut ());
    }
  }
}

impl Debug for Any {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    unsafe {
      let debug_fmt = self.vtable().debug_fmt;
      let this = self as *const Any as *const ();
      (debug_fmt)(this, f)
    }
  }
}

impl Display for Any {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    unsafe {
      let display_fmt = self.vtable().display_fmt;
      let this = self as *const Any as *const ();
      (display_fmt)(this, f)
    }
  }
}

impl<T: Sized + 'static> Ptr<T> {
  pub fn into_any(self) -> Ptr<Any> {
    unsafe { mem::transmute::<Ptr<T>, Ptr<Any>>(self) }
  }
}

impl Ptr<Any> {
  pub fn is<T: Type>(&self) -> bool {
    self.repr().type_id == TypeId::of::<T>()
  }

  pub fn cast<T: Type>(self) -> Result<Ptr<T>, Ptr<Any>> {
    match self.is::<T>() {
      true => Ok(unsafe { self.cast_unchecked() }),
      false => Err(self),
    }
  }

  pub fn clone_cast<T: Type>(&self) -> Option<Ptr<T>> {
    self.clone().cast().ok()
  }

  /// # Safety
  /// - `self.is::<T>()` must be `true`
  pub unsafe fn cast_unchecked<T: Type>(self) -> Ptr<T> {
    debug_assert!(
      self.is::<T>(),
      "object is not an instance of {}",
      std::any::type_name::<T>()
    );
    mem::transmute::<Ptr<Any>, Ptr<T>>(self)
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::rc::Rc;

  use super::*;
  use crate::object::Object;

  struct Foo {
    value: i32,
    on_drop: Box<dyn FnMut()>,
  }

  impl Object for Foo {
    fn type_name(_: Ptr<Self>) -> &'static str {
      "Foo"
    }
  }

  declare_object_type!(Foo);

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
    value: i32,
  }

  impl Object for Bar {
    fn type_name(_: Ptr<Self>) -> &'static str {
      "Bar"
    }
  }

  declare_object_type!(Bar);

  impl Display for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      Debug::fmt(self, f)
    }
  }

  fn noop() {}

  #[allow(clippy::redundant_clone)]
  #[test]
  fn object_repr_refcount() {
    let global = Global::default();

    let foo = global.alloc(Foo {
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
    let global = Global::default();

    let foo = global
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
    let global = Global::default();

    let foo = global.alloc(Foo {
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
    let global = Global::default();

    // static
    {
      let dropped = Rc::new(RefCell::new(false));
      let foo = global.alloc(Foo {
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
      let foo = global.alloc(Foo {
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
    let cx = Global::default();

    let v = cx.alloc(Bar { value: 100 });
    let v = v.into_any();
    let v = v.cast::<Foo>().unwrap_err();
    let _ = v.cast::<Bar>().unwrap();
  }

  #[test]
  fn debug_and_display_fmt() {
    let global = Global::default();

    let v = global.alloc(Bar { value: 100 });
    assert_eq!("Bar { value: 100 }", v.to_string());
    let v = v.into_any();
    assert_eq!("Bar { value: 100 }", v.to_string());
  }
}
