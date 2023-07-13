//! # Garbage collector
//!
//! The VM uses an incremental, generational, mark-and-sweep garbage collector
//! which is designed to be cache-friendly, have low memory overhead, and
//! minimise pause times.
//!
//! The garbage collector supports two modes:
//! - `frame`
//! - `trace`
//!
//! ## Frame mode
//!
//! This mode is intended for use-cases in which you create a new VM instance
//! for each run of a Hebi program.
//!
//! In this mode, memory allocations will never trigger a collection cycle.
//! This means that the heap will continue to grow, and no memory will ever
//! be reclaimed while the VM lives. All memory is properly freed once the VM is
//! dropped.
//!
//! This means it completely unsuitable for anything long running- be careful,
//! as you may find yourself easily running out of memory, or having your
//! program slow down to a halt due to the OS paging memory in and out of disk.
//!
//! ## Trace mode
//!
//! This mode is intended for longer-running tasks, where you'd otherwise have a
//! chance of running out of memory without any memory reclamation mechanism.
//! Example use-cases include the main loop of a game, or a TCP listener accept
//! loop.
//!
//! Memory is allocated and managed by a proper garbage collector. A memory
//! allocation may trigger a collection cycle.
//!
//! The garbage collector is an implementation of [this design](http://web.archive.org/web/20220107060536/http://wiki.luajit.org/New-Garbage-Collector)
//! by Mike Pall, originally intended for use in LuaJIT 3.
//!
//! NOTE: For the time being, the garbage collector uses a simpler tri-color
//! incremental algorithm. The plan is to eventually implement the full
//! quad-color algorithm.

use std::any::type_name;
use std::cell::Cell;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::{mem, ptr};

use bumpalo::Bump;

pub trait Object: Debug + Display {}

// TODO:
pub struct Value {
  _v: u64,
}

pub trait Tracer: private::Sealed {
  fn object<T: Object + 'static>(&self, obj: Ref<T>);
  fn value(&self, value: Value);
}

/// # Safety
/// You must ensure that all fields which hold values or objects are passed to
/// the `tracer` instance.
///
/// This type should be automatically derived using the `#[object]` attribute
/// whenever possible. `#[object]` will also insert the correct write barriers.
pub unsafe trait Trace {
  fn trace(&self, tracer: &impl Tracer);
}

mod private {
  pub trait Sealed {}
}

pub struct Gc {
  heap: Bump,
  drop_chain: Cell<Option<NonNull<Box<()>>>>,

  /// Ensure that `Gc` is not `Send` or `Sync`
  _not_thread_safe: PhantomData<()>,
}

impl Gc {
  pub fn new() -> Self {
    Gc::default()
  }

  pub fn with_base_capacity(capacity: usize) -> Self {
    Gc {
      heap: Bump::with_capacity(capacity),
      drop_chain: Cell::new(None),
      _not_thread_safe: PhantomData,
    }
  }
}

impl Default for Gc {
  fn default() -> Self {
    Gc::with_base_capacity(4096)
  }
}

impl Gc {
  pub fn alloc<T: Object + 'static>(&self, obj: T) -> Ref<T> {
    if mem::needs_drop::<T>() {
      let ptr = NonNull::from(self.heap.alloc(Box {
        next: self.drop_chain.get(),
        info: vtable::<T>(),
        data: obj,
      }));
      self.drop_chain.replace(Some(ptr.cast()));
      Ref { ptr }
    } else {
      Ref {
        ptr: NonNull::from(self.heap.alloc(Box {
          next: None,
          info: vtable::<T>(),
          data: obj,
        })),
      }
    }
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
pub struct Any {
  /// Type-erased pointer to the object
  ptr: NonNull<Box<()>>,
}

impl Any {
  /// Checks if `self` is a reference to a `T`.
  #[inline(always)]
  pub fn is<T: Object + 'static>(&self) -> bool {
    ptr::eq(vtable::<T>().erase(), self.vtable())
  }

  /// Converts `self` into a reference to type `T`.
  ///
  /// Returns `None` if this reference does not point to a `T`.
  #[inline(always)]
  pub fn cast<T: Object + 'static>(self) -> Option<Ref<T>> {
    if self.is::<T>() {
      Some(unsafe { self.coerce() })
    } else {
      None
    }
  }

  #[inline(always)]
  pub fn addr(self) -> usize {
    self.ptr.as_ptr() as usize
  }

  /// Reconstructs an object reference from its address.
  ///
  /// # Safety
  /// The resulting pointer must be valid and non-null.
  ///
  /// The easiest way to ensure that is to allocate it via
  /// `obj = gc.alloc(...)`, and then use `obj.addr()`.
  #[inline(always)]
  pub unsafe fn from_addr(addr: usize) -> Any {
    unsafe {
      Any {
        ptr: NonNull::new_unchecked(addr as *mut Box<()>),
      }
    }
  }

  /// Converts `self` into a reference to type `T`.
  ///
  /// This is the same as `self.cast::<T>()`, but this version
  /// does not check if `self.is::<T>()`.
  ///
  /// # Safety
  /// `self.is::<T>()` must be `true`.
  #[inline(always)]
  pub unsafe fn coerce<T: Object + 'static>(self) -> Ref<T> {
    debug_assert!(self.is::<T>(), "invalid cast to Ref<{}>", type_name::<T>());
    Ref {
      ptr: self.ptr.cast(),
    }
  }

  #[inline(always)]
  fn vtable(&self) -> &'static VTable<()> {
    unsafe { ptr::addr_of!((*self.ptr.as_ptr()).info).read() }
  }

  #[inline(always)]
  fn data(&self) -> *const () {
    unsafe { ptr::addr_of!((*self.ptr.as_ptr()).data) }
  }
}

#[derive(Clone, Copy)]
pub struct Ref<T: 'static> {
  ptr: NonNull<Box<T>>,
}

impl<T> Ref<T> {
  #[inline]
  pub fn erase(self) -> Any {
    Any {
      ptr: self.ptr.cast(),
    }
  }
}

impl<T> Deref for Ref<T> {
  type Target = T;

  #[inline]
  fn deref(&self) -> &Self::Target {
    unsafe { &self.ptr.as_ref().data }
  }
}

impl Display for Any {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    (self.vtable().display_fmt)(self.data(), f)
  }
}

impl Debug for Any {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    (self.vtable().debug_fmt)(self.data(), f)
  }
}

impl<T: Object> Display for Ref<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(self.deref(), f)
  }
}

impl<T: Object> Debug for Ref<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(self.deref(), f)
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
  display_fmt: fn(*const T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
  debug_fmt: fn(*const T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
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
    display_fmt: |p, f| {
      // Safety:
      // `p` is guaranteed to be non-null and valid for reads.
      // See `NonNull` in `Any` and `Ref<T>`
      <T as std::fmt::Display>::fmt(unsafe { p.as_ref().unwrap_unchecked() }, f)
    },
    debug_fmt: |p, f| {
      // Safety:
      // `p` is guaranteed to be non-null and valid for reads.
      // See `NonNull` in `Any` and `Ref<T>`
      <T as std::fmt::Debug>::fmt(unsafe { p.as_ref().unwrap_unchecked() }, f)
    },
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
  fn debug_and_display() {
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
    assert_eq!("Foo { n: 10 }", format!("{v:?}"));
    assert_eq!("<foo 10>", format!("{v}"));

    let v = v.erase();
    assert_eq!("Foo { n: 10 }", format!("{v:?}"));
    assert_eq!("<foo 10>", format!("{v}"));
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
