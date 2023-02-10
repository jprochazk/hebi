pub use std::cell::{Ref, RefCell, RefMut};
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone)]
pub struct Ptr<T>(pub(crate) Rc<RefCell<T>>);

impl<T> Ptr<T> {
  pub fn new(v: T) -> Self {
    Ptr(Rc::new(RefCell::new(v)))
  }

  /// Safety:
  /// - `addr` must come from `Ptr::into_addr`, and the underlying memory must
  ///   still be live
  pub(crate) unsafe fn from_addr(addr: usize) -> Self {
    Ptr(Rc::from_raw(addr as *const RefCell<T>))
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
    unsafe { Rc::<RefCell<T>>::increment_strong_count(ptr) }
  }

  pub(crate) unsafe fn decrement_strong_count(addr: usize) {
    let ptr = addr as *const _;
    unsafe { Rc::<RefCell<T>>::decrement_strong_count(ptr) }
  }
}

impl<T> Deref for Ptr<T> {
  type Target = RefCell<T>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T: PartialEq> PartialEq for Ptr<T> {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}
impl<T: Eq> Eq for Ptr<T> {}

impl<T: Hash> Hash for Ptr<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.borrow().hash(state);
  }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <T as std::fmt::Debug>::fmt(&self.0.borrow(), f)
  }
}

impl<T: std::fmt::Display> std::fmt::Display for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <T as std::fmt::Display>::fmt(&self.0.borrow(), f)
  }
}

// TODO: improve portability by adding a fallback to a `Value` enum

// this asserts that `Ptr` is 64 bits,
// which it should be on systems where `usize == u64`
// `Value` doesn't work on 32-bit systems, so this doubles
// as an architecture check
const _: fn() = || {
  let _ = std::mem::transmute::<Ptr<crate::object::Object>, u64>;
};
