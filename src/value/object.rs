/*

*/

use std::alloc::Layout;
use std::any::{Any as DynAny, TypeId};
use std::cell::Cell;
use std::ops::Deref;
use std::ptr::{self, NonNull, Pointee};
use std::{alloc, mem};

use crate::ctx::Context;

pub struct Value {}

pub trait Object: DynAny {
  fn name(&self, cx: Context) -> &'static str;
  fn get_field(&self, cx: Context, key: &str) -> Option<Value>;
  fn set_field(&self, cx: Context, key: &str, value: Value);
  /* fn get_index(&self, key: Value) -> Option<Value>;
  fn set_index(&self, key: Value, value: Value); */
}

type VTable = <dyn Object as Pointee>::Metadata;

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

  pub(crate) fn refs(&self) -> u64 {
    self.inner().refs.get()
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
      self.inner().refs.set(self.refs() - 1);
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
    self.inner().refs.set(self.inner().refs.get() + 1);
    Self { repr: self.repr }
  }
}

impl<T: Object> Object for Ptr<T> {
  fn name(&self, cx: Context) -> &'static str {
    self.inner().data.name(cx)
  }

  fn get_field(&self, cx: Context, key: &str) -> Option<Value> {
    self.inner().data.get_field(cx, key)
  }

  fn set_field(&self, cx: Context, key: &str, value: Value) {
    self.inner().data.set_field(cx, key, value)
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
  fn name(&self, cx: Context) -> &'static str {
    let this = unsafe { self.as_dyn_object() };
    this.name(cx)
  }

  fn get_field(&self, cx: Context, key: &str) -> Option<Value> {
    let this = unsafe { self.as_dyn_object() };
    this.get_field(cx, key)
  }

  fn set_field(&self, cx: Context, key: &str, value: Value) {
    let this = unsafe { self.as_dyn_object() };
    this.set_field(cx, key, value)
  }
}

impl<T: Object> Ptr<T> {
  fn as_any(self) -> Ptr<Any> {
    unsafe { mem::transmute::<Ptr<T>, Ptr<Any>>(self) }
  }
}

impl Ptr<Any> {
  fn cast<T: Object>(self) -> Option<Ptr<T>> {
    match self.inner().type_id == TypeId::of::<T>() {
      true => Some(unsafe { mem::transmute::<Ptr<Any>, Ptr<T>>(self) }),
      false => None,
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
    fn name(&self, cx: Context) -> &'static str {
      let _ = cx;
      "Foo"
    }

    fn get_field(&self, cx: Context, key: &str) -> Option<Value> {
      let _ = cx;
      let _ = key;
      None
    }

    fn set_field(&self, cx: Context, key: &str, value: Value) {
      let _ = cx;
      let _ = key;
      let _ = value;
    }
  }

  impl Drop for Foo {
    fn drop(&mut self) {
      (self.on_drop)();
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
      .as_any();
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
    let foo = foo.as_any();
    assert_eq!(foo.name(cx), "Foo");
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
      let foo = foo.as_any();
      drop(foo);
      assert!(*dropped.borrow());
    }
  }
}
