use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::{mem, ptr};

use bumpalo::Bump;

use super::{Collector, Object};

pub struct Gc {
  heap: Bump,
  drop_chain: Cell<Option<NonNull<Box<()>>>>,

  /// Ensure that `Gc` is not `Send` or `Sync`
  _not_thread_safe: PhantomData<()>,
}

impl Collector for Gc {
  type Typed<T: Object + 'static> = TObj<T>;
  type Untyped = Obj;

  fn alloc<T: Object + 'static>(&self, obj: T) -> TObj<T> {
    if mem::needs_drop::<T>() {
      let obj = TObj::new(self.alloc_needs_drop(obj));
      self.drop_chain.replace(Some(obj.ptr.cast()));
      obj
    } else {
      TObj::new(self.alloc_pod(obj))
    }
  }

  fn write<T: Object + 'static>(obj: &T) {
    // The bump GC does not trace anything.
  }

  fn write_container<T: Object + 'static>(obj: &T) {
    // The bump GC does not trace anything.
  }
}

impl Gc {
  pub fn new() -> Self {
    Gc {
      heap: Bump::new(),
      drop_chain: Cell::new(None),
      _not_thread_safe: PhantomData,
    }
  }

  #[inline]
  fn alloc_pod<T: Object>(&self, obj: T) -> NonNull<Box<T>> {
    NonNull::from(self.heap.alloc(Box {
      next: None,
      info: vtable::<T>(),
      data: obj,
    }))
  }

  #[inline]
  fn alloc_needs_drop<T: Object>(&self, obj: T) -> NonNull<Box<T>> {
    NonNull::from(self.heap.alloc(Box {
      next: self.drop_chain.get(),
      info: vtable::<T>(),
      data: obj,
    }))
  }
}

impl Drop for Gc {
  fn drop(&mut self) {
    unsafe {
      // call the drop impls of every object in the `finalized` linked list
      let mut ptr = self.drop_chain.get();
      while let Some(v) = ptr {
        let v = v.as_ptr();
        ptr = ptr::addr_of!((*v).next).read();
        let drop_in_place = ptr::addr_of!((*v).info).read().drop_in_place;
        let data = ptr::addr_of_mut!((*v).data);
        drop_in_place(data);
      }
    }
  }
}

#[derive(Clone, Copy)]
pub struct Obj {
  /// Type-erased pointer to the object
  ptr: NonNull<Box<()>>,
}

impl Obj {
  #[inline]
  pub fn cast<T: Object + 'static>(self) -> Option<TObj<T>> {
    if ptr::eq(vtable::<T>().erase(), self.vtable()) {
      Some(unsafe { self.cast_unchecked() })
    } else {
      None
    }
  }

  #[inline(always)]
  pub unsafe fn cast_unchecked<T: Object + 'static>(self) -> TObj<T> {
    TObj {
      ptr: self.ptr.cast(),
    }
  }

  #[inline(always)]
  fn vtable(&self) -> &'static VTable<()> {
    unsafe { ptr::addr_of!((*self.ptr.as_ptr()).info).read() }
  }
}

#[derive(Clone, Copy)]
pub struct TObj<T: 'static> {
  ptr: NonNull<Box<T>>,
}

impl<T> TObj<T> {
  fn new(ptr: NonNull<Box<T>>) -> Self {
    Self { ptr }
  }

  pub fn erase(self) -> Obj {
    Obj {
      ptr: self.ptr.cast(),
    }
  }
}

impl<T> Deref for TObj<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { &self.ptr.as_ref().data }
  }
}

#[repr(C)]
struct Box<T: 'static> {
  /// Linked list of all finalized objects
  next: Option<NonNull<Box<()>>>,
  info: &'static VTable<T>,
  data: T,
}

const fn vtable<T: HasVTable<T>>() -> &'static VTable<T> {
  T::VTABLE
}

#[doc(hidden)]
struct VTable<T> {
  drop_in_place: unsafe fn(*mut T),
}

impl<T> VTable<T> {
  const fn erase(&'static self) -> &'static VTable<()> {
    unsafe { mem::transmute(self) }
  }
}

trait HasVTable<T: 'static> {
  const VTABLE: &'static VTable<T>;
}
impl<T: Sized + Object + 'static> HasVTable<T> for T {
  const VTABLE: &'static VTable<T> = &VTable {
    drop_in_place: ptr::drop_in_place::<T>,
  };
}

#[cfg(test)]
mod tests {
  use std::fmt::{Debug, Display};

  use super::*;

  #[test]
  fn type_cast() {
    #[derive(Debug)]
    struct Foo {
      n: u32,
    }
    impl Object for Foo {}
    impl Display for Foo {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<foo {}>", self.n)
      }
    }

    let gc = Gc::new();
    let v = gc.alloc(Foo { n: 10 });
    assert_eq!(v.n, 10);
    let v = v.erase();
    let v = v.cast::<Foo>().unwrap();
    assert_eq!(v.n, 10);
  }

  #[test]
  fn finalization() {
    #[allow(dead_code)]
    struct NotDropped {
      dropped: *mut bool,
    }

    struct Dropped {
      dropped: *mut bool,
    }

    impl Drop for Dropped {
      fn drop(&mut self) {
        unsafe { self.dropped.write(true) }
      }
    }

    impl Object for NotDropped {}
    impl Object for Dropped {}
    impl Display for NotDropped {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<not dropped>")
      }
    }
    impl Display for Dropped {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<dropped>")
      }
    }
    impl Debug for NotDropped {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotDropped").finish()
      }
    }
    impl Debug for Dropped {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dropped").finish()
      }
    }

    let mut d0 = false;
    let mut d1 = false;
    {
      let gc = Gc::new();
      let _ = gc.alloc(NotDropped {
        dropped: &mut d0 as _,
      });
      let _ = gc.alloc(Dropped {
        dropped: &mut d1 as _,
      });
      drop(gc);
    }

    assert!(!d0, "0 was dropped");
    assert!(d1, "1 was not dropped");
  }
}
