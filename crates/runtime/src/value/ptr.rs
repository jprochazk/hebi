use std::cell::UnsafeCell;
use std::rc::Rc;

use super::object;

#[derive(Clone)]
pub struct Ptr<T>(pub(crate) Rc<UnsafeCell<T>>);

impl<T> Ptr<T> {
  pub(crate) fn alloc(v: T) -> Self {
    Ptr(Rc::new(UnsafeCell::new(v)))
  }

  /// Do not use directly.
  #[doc(hidden)]
  pub(crate) unsafe fn _get(&self) -> &T {
    unsafe { self.0.get().as_ref().unwrap_unchecked() }
  }

  /// Do not use directly.
  #[doc(hidden)]
  pub(crate) unsafe fn _get_mut(&mut self) -> &mut T {
    unsafe { self.0.get().as_mut().unwrap_unchecked() }
  }

  /// Safety:
  /// - `addr` must come from `Ptr::into_addr`, and the underlying memory must
  ///   still be live
  pub(crate) unsafe fn from_addr(addr: usize) -> Self {
    Ptr(Rc::from_raw(addr as *const UnsafeCell<T>))
  }

  /// To avoid a memory leak, the address must be converted back into a `Ptr`
  /// using `from_addr`.
  pub(crate) fn into_addr(this: Ptr<T>) -> usize {
    Rc::into_raw(this.0) as usize
  }

  #[allow(dead_code)]
  pub(crate) fn strong_count(this: &Ptr<T>) -> usize {
    Rc::strong_count(&this.0)
  }

  #[allow(dead_code)]
  pub(crate) fn weak_count(this: &Ptr<T>) -> usize {
    Rc::weak_count(&this.0)
  }

  pub(crate) unsafe fn increment_strong_count(addr: usize) {
    let ptr = addr as *const _;
    unsafe { Rc::<UnsafeCell<T>>::increment_strong_count(ptr) }
  }

  pub(crate) unsafe fn decrement_strong_count(addr: usize) {
    let ptr = addr as *const _;
    unsafe { Rc::<UnsafeCell<T>>::decrement_strong_count(ptr) }
  }
}

// TODO: improve portability by adding a fallback to a `Value` enum

// this asserts that `Ptr` is 64 bits,
// which it should be on systems where `usize == u64`
// `Value` doesn't work on 32-bit systems, so this doubles
// as an architecture check
const _: fn() = || {
  let _ = std::mem::transmute::<Ptr<object::Object>, u64>;
};
