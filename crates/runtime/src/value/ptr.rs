use std::cell::UnsafeCell;
use std::hash::Hash;
use std::rc::Rc;

use super::object;

#[derive(Clone)]
pub struct Ptr<T>(pub(crate) Rc<UnsafeCell<T>>);

impl<T> Ptr<T> {
  pub fn new(v: T) -> Self {
    Ptr(Rc::new(UnsafeCell::new(v)))
  }

  pub fn get(&self) -> &T {
    unsafe { self.0.get().as_ref().unwrap_unchecked() }
  }

  // TODO: this is totally unsound if you do:
  // ```
  // let mut v0 = Ptr::new(#[may_alias] 0u32);
  // let mut v1 = v0.clone();
  // let r0 = v0.deref_mut();
  // let r1 = v1.deref_mut(); // immediate UB.
  // r0.add_assign(*r1);
  // ```
  // but usage is awful without this.
  pub fn get_mut(&mut self) -> &mut T {
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

  pub fn strong_count(this: &Ptr<T>) -> usize {
    Rc::strong_count(&this.0)
  }

  pub fn weak_count(this: &Ptr<T>) -> usize {
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

impl<T: PartialEq> PartialEq for Ptr<T> {
  fn eq(&self, other: &Self) -> bool {
    self.get() == other.get()
  }
}
impl<T: Eq> Eq for Ptr<T> {}

impl<T: Hash> Hash for Ptr<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.get().hash(state);
  }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <T as std::fmt::Debug>::fmt(self.get(), f)
  }
}

impl<T: std::fmt::Display> std::fmt::Display for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <T as std::fmt::Display>::fmt(self.get(), f)
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
