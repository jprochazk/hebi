#![allow(clippy::needless_lifetimes)]

use core::alloc::Layout;
use core::any::type_name;
use core::borrow::Borrow;
use core::cell::{Cell, UnsafeCell};
use core::cmp::max;
use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::marker::PhantomData;
use core::mem::transmute;
use core::ops::Deref;
use core::ptr::{addr_of, addr_of_mut, copy_nonoverlapping, NonNull};
use core::{mem, ptr, slice, str};

use allocator_api2::alloc::Allocator;
use bumpalo::{vec, Bump};

use crate::ds::fx;
use crate::ds::set::GcHashSet;
use crate::error::AllocError;
use crate::val::Value;

pub trait Object: Debug + Display {
  /// Whether or not `Self` needs to have its `Drop` impl called.
  ///
  /// This has a default implementation using `core::mem::needs_drop::<Self>`,
  /// which does the right thing in most cases, but it can be overridden in
  /// case the default is incorrect.
  ///
  /// For example, this always returns `false` for `List`, because we don't
  /// want the underlying `Vec` to drop its contents, as they will be GC'd
  /// automatically, and the GC is responsible for calling `Drop` impls.
  const NEEDS_DROP: bool = core::mem::needs_drop::<Self>();
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
  drop_chain: Cell<Option<NonNull<GcBox<()>>>>,
  string_table: NonNull<UnsafeCell<GcHashSet<'static, &'static str>>>,

  /// Ensure that `Gc` is not `Send` or `Sync`
  _not_thread_safe: PhantomData<()>,
}

impl Gc {
  pub fn new() -> Self {
    Gc::default()
  }

  pub fn with_base_capacity(capacity: usize) -> Self {
    let heap = Bump::with_capacity(capacity);
    let string_table = GcHashSet::with_hasher_in(fx(), Alloc::null());
    let string_table = NonNull::from(heap.alloc(UnsafeCell::new(string_table)));
    Gc {
      drop_chain: Cell::new(None),
      string_table,
      heap,
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
  pub fn try_alloc<T: Object + 'static>(&self, obj: T) -> Result<Ref<T>, AllocError> {
    let ptr = if T::NEEDS_DROP {
      let ptr = self.heap.try_alloc(GcBox {
        next: self.drop_chain.get(),
        info: vtable::<T>(),
        data: obj,
      })?;
      let ptr = NonNull::from(ptr);
      self.drop_chain.replace(Some(ptr.cast()));
      ptr
    } else {
      NonNull::from(self.heap.try_alloc(GcBox {
        next: None,
        info: vtable::<T>(),
        data: obj,
      })?)
    };

    Ok(Ref { ptr })
  }

  pub fn try_alloc_str<'gc>(&'gc self, src: &str) -> Result<&'gc str, AllocError> {
    let dst = self.heap.try_alloc_layout(Layout::for_value(src))?;
    let dst = unsafe {
      copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
      slice::from_raw_parts(dst.as_ptr(), src.len())
    };
    Ok(unsafe { str::from_utf8_unchecked(dst) })
  }

  #[inline]
  unsafe fn string_table(&self) -> &GcHashSet<'static, &'static str> {
    &*self.string_table.as_ref().get()
  }

  #[allow(clippy::mut_from_ref)]
  #[inline]
  unsafe fn string_table_mut(&self) -> &mut GcHashSet<'static, &'static str> {
    &mut *self.string_table.as_ref().get()
  }

  pub fn try_intern_str<'gc>(&'gc self, src: &str) -> Result<&'gc str, AllocError> {
    if let Some(str) = unsafe { self.string_table() }.get(src).copied() {
      return Ok(str);
    }
    let str = self.try_alloc_str(src)?;
    let str = unsafe { transmute::<&'gc str, &'static str>(str) };
    let table = unsafe { self.string_table_mut() };
    table.allocator().set(self);
    table.insert(str);
    Ok(str)
  }

  /// Allocate a slice on the GC heap.
  ///
  /// The contents of the slice will _not_ be dropped.
  pub fn try_alloc_slice<'gc, T>(&'gc self, src: &[T]) -> Result<&'gc [T], AllocError> {
    let dst = self.heap.try_alloc_layout(Layout::for_value(src))?;
    let dst = dst.cast::<T>();
    let dst = unsafe {
      copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
      slice::from_raw_parts(dst.as_ptr(), src.len())
    };
    Ok(dst)
  }

  pub fn try_collect_slice<'gc, T>(
    &'gc self,
    items: impl IntoIterator<Item = Result<T, AllocError>>,
  ) -> Result<&'gc [T], AllocError> {
    let mut iter = items.into_iter();
    let size_hint = iter.size_hint();
    let len = max(size_hint.1.unwrap_or(size_hint.0), 1);
    let mut v = vec![in &self.heap];
    v.try_reserve(len).map_err(|_| AllocError)?;
    if let Some(item) = iter.next() {
      let item = item?;
      // we have space for at least one item
      v.push(item);
    }
    for item in iter {
      let item = item?;
      v.try_reserve(1).map_err(|_| AllocError)?;
      v.push(item);
    }
    Ok(v.into_bump_slice())
  }
}

impl Drop for Gc {
  fn drop(&mut self) {
    unsafe {
      // call the drop impls of every object in the `finalized` linked list
      let mut ptr = self.drop_chain.get();
      while let Some(v) = ptr {
        let v = v.as_ptr();
        ptr = addr_of!((*v).next).read();
        let drop_in_place = addr_of!((*v).info).read().drop_in_place;
        let data = addr_of_mut!((*v).data);
        drop_in_place(data);
      }
    }
  }
}

#[derive(Clone, Copy)]
pub struct Any {
  /// Type-erased pointer to the object
  ptr: NonNull<GcBox<()>>,
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
        ptr: NonNull::new_unchecked(addr as *mut GcBox<()>),
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

pub struct Ref<T: 'static> {
  ptr: NonNull<GcBox<T>>,
}

impl<T> Copy for Ref<T> {}
impl<T> Clone for Ref<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Ref<T> {
  #[inline]
  pub fn erase(self) -> Any {
    Any {
      ptr: self.ptr.cast(),
    }
  }

  #[inline]
  pub fn get_ref(&self) -> &T {
    unsafe { &self.ptr.as_ref().data }
  }
}

impl<T> Deref for Ref<T> {
  type Target = T;

  #[inline]
  fn deref(&self) -> &Self::Target {
    self.get_ref()
  }
}

impl Display for Any {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    (self.vtable().display_fmt)(self.data(), f)
  }
}

impl Debug for Any {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    (self.vtable().debug_fmt)(self.data(), f)
  }
}

impl<T: Object> Display for Ref<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Display::fmt(self.deref(), f)
  }
}

impl<T: Object> Debug for Ref<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Debug::fmt(self.deref(), f)
  }
}

impl<T: Hash> Hash for Ref<T> {
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    self.get_ref().hash(state)
  }
}

impl<T: PartialEq> PartialEq for Ref<T> {
  fn eq(&self, other: &Ref<T>) -> bool {
    self.get_ref().eq(other.get_ref())
  }
}
impl<T: Eq> Eq for Ref<T> {}

impl<T: PartialOrd> PartialOrd for Ref<T> {
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    self.get_ref().partial_cmp(other.get_ref())
  }
}
impl<T: Ord> Ord for Ref<T> {
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.get_ref().cmp(other.get_ref())
  }
}

impl<T> Borrow<T> for Ref<T> {
  fn borrow(&self) -> &T {
    self.deref()
  }
}

#[repr(C)]
struct GcBox<T: 'static> {
  /// Linked list of all finalized objects
  next: Option<NonNull<GcBox<()>>>,
  info: &'static VTable<T>,
  data: T,
}

const fn vtable<T: HasVTable<T>>() -> &'static VTable<T> {
  T::VTABLE
}

#[doc(hidden)]
struct VTable<T> {
  drop_in_place: unsafe fn(*mut T),
  display_fmt: fn(*const T, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
  debug_fmt: fn(*const T, &mut core::fmt::Formatter<'_>) -> core::fmt::Result,
}

impl<T> VTable<T> {
  const fn erase(&'static self) -> &'static VTable<()> {
    unsafe { mem::transmute(self) }
  }
}

trait HasVTable<T: 'static> {
  const VTABLE: &'static VTable<T>;
}
impl<T: Object + 'static> HasVTable<T> for T {
  const VTABLE: &'static VTable<T> = &VTable {
    drop_in_place: ptr::drop_in_place::<T>,
    display_fmt: |p, f| {
      // Safety:
      // `p` is guaranteed to be non-null and valid for reads.
      // See `NonNull` in `Any` and `Ref<T>`
      <T as core::fmt::Display>::fmt(unsafe { p.as_ref().unwrap_unchecked() }, f)
    },
    debug_fmt: |p, f| {
      // Safety:
      // `p` is guaranteed to be non-null and valid for reads.
      // See `NonNull` in `Any` and `Ref<T>`
      <T as core::fmt::Debug>::fmt(unsafe { p.as_ref().unwrap_unchecked() }, f)
    },
  };
}

impl Gc {
  #[inline(always)]
  fn allocator(&self) -> impl Allocator + '_ {
    &self.heap
  }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NoAlloc(usize, PhantomData<&'static ()>);
pub const NO_ALLOC: NoAlloc = NoAlloc(0, PhantomData);
unsafe impl Allocator for NoAlloc {
  fn allocate(
    &self,
    _: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    unreachable!()
  }

  unsafe fn deallocate(&self, _: NonNull<u8>, _: core::alloc::Layout) {
    unreachable!()
  }

  fn allocate_zeroed(
    &self,
    _: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    unreachable!()
  }

  unsafe fn grow(
    &self,
    _: NonNull<u8>,
    _: core::alloc::Layout,
    _: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    unreachable!()
  }

  unsafe fn grow_zeroed(
    &self,
    _: NonNull<u8>,
    _: core::alloc::Layout,
    _: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    unreachable!()
  }

  unsafe fn shrink(
    &self,
    _: NonNull<u8>,
    _: core::alloc::Layout,
    _: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    unreachable!()
  }

  fn by_ref(&self) -> &Self
  where
    Self: Sized,
  {
    self
  }
}

#[derive(Clone)]
#[repr(C)]
pub struct Alloc<'gc>(Cell<*const Gc>, PhantomData<&'gc ()>);

impl Alloc<'static> {
  fn null() -> Self {
    Self(Cell::new(ptr::null()), PhantomData)
  }
}

impl<'gc> Alloc<'gc> {
  pub fn new(gc: &'gc Gc) -> Self {
    Self(Cell::new(gc), PhantomData)
  }

  pub fn set(&self, gc: &'gc Gc) {
    self.0.set(gc);
  }

  fn inner(&self) -> &'gc Gc {
    unsafe { self.0.get().as_ref().unwrap_unchecked() }
  }
}

unsafe impl<'gc> Allocator for Alloc<'gc> {
  fn allocate(
    &self,
    layout: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    self.inner().allocator().allocate(layout)
  }

  unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: core::alloc::Layout) {
    self.inner().allocator().deallocate(ptr, layout)
  }

  fn allocate_zeroed(
    &self,
    layout: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    self.inner().allocator().allocate_zeroed(layout)
  }

  unsafe fn grow(
    &self,
    ptr: NonNull<u8>,
    old_layout: core::alloc::Layout,
    new_layout: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    self.inner().allocator().grow(ptr, old_layout, new_layout)
  }

  unsafe fn grow_zeroed(
    &self,
    ptr: NonNull<u8>,
    old_layout: core::alloc::Layout,
    new_layout: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    self
      .inner()
      .allocator()
      .grow_zeroed(ptr, old_layout, new_layout)
  }

  unsafe fn shrink(
    &self,
    ptr: NonNull<u8>,
    old_layout: core::alloc::Layout,
    new_layout: core::alloc::Layout,
  ) -> Result<NonNull<[u8]>, allocator_api2::alloc::AllocError> {
    self.inner().allocator().shrink(ptr, old_layout, new_layout)
  }

  fn by_ref(&self) -> &Self
  where
    Self: Sized,
  {
    self
  }
}

#[cfg(test)]
mod tests {
  use core::fmt::{Debug, Display};

  use super::*;
  use crate::alloc::format;
  use crate::util::static_assert;

  #[test]
  fn type_cast() {
    #[derive(Debug)]
    struct Foo {
      n: u32,
    }
    impl Object for Foo {}
    impl Display for Foo {
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<foo {}>", self.n)
      }
    }

    let gc = Gc::new();
    let v = gc.try_alloc(Foo { n: 10 }).unwrap();
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
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<foo {}>", self.n)
      }
    }

    let gc = Gc::new();
    let v = gc.try_alloc(Foo { n: 10 }).unwrap();
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
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<not dropped>")
      }
    }
    impl Display for Dropped {
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<dropped>")
      }
    }
    impl Debug for NotDropped {
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NotDropped").finish()
      }
    }
    impl Debug for Dropped {
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Dropped").finish()
      }
    }

    let mut d0 = false;
    let mut d1 = false;
    {
      let gc = Gc::new();
      let _ = gc
        .try_alloc(NotDropped {
          dropped: &mut d0 as _,
        })
        .unwrap();
      let _ = gc
        .try_alloc(Dropped {
          dropped: &mut d1 as _,
        })
        .unwrap();
      drop(gc);
    }

    assert!(!d0, "0 was dropped");
    assert!(d1, "1 was not dropped");
  }

  fn _assert_needs_drop() {
    #[derive(Debug)]
    struct Dropped {}
    impl Drop for Dropped {
      fn drop(&mut self) {
        unimplemented!()
      }
    }

    #[derive(Debug)]
    struct NoDropPod(i32);
    impl Object for NoDropPod {}
    impl Display for NoDropPod {
      fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
      }
    }

    #[derive(Debug)]
    struct PropagateDrop(Dropped);
    impl Object for PropagateDrop {}
    impl Display for PropagateDrop {
      fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
      }
    }

    #[derive(Debug)]
    struct ForceNoDrop(Dropped);
    impl Object for ForceNoDrop {
      const NEEDS_DROP: bool = false;
    }
    impl Display for ForceNoDrop {
      fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        unimplemented!()
      }
    }

    const _: () = static_assert(!NoDropPod::NEEDS_DROP, "failed drop check");
    const _: () = static_assert(PropagateDrop::NEEDS_DROP, "failed drop check");
    const _: () = static_assert(!ForceNoDrop::NEEDS_DROP, "failed drop check");
  }
}
