use std::rc::Rc;

#[derive(Clone)]
pub struct Ptr<T>(pub(crate) Rc<T>);

impl<T> Ptr<T> {
  pub fn new(v: T) -> Self {
    Ptr(Rc::new(v))
  }

  /// Safety:
  /// - `addr` must come from `Ptr::into_addr`, and the underlying memory must
  ///   still be live
  pub(crate) unsafe fn from_addr(addr: usize) -> Self {
    Ptr(Rc::from_raw(addr as *const T))
  }

  /// To avoid a memory leak, the address must be converted back into a `Ptr`
  /// using `from_addr`.
  pub(crate) fn into_addr(this: Ptr<T>) -> usize {
    Rc::into_raw(this.0) as usize
  }

  pub(crate) fn strong_count(this: &Ptr<T>) -> usize {
    Rc::strong_count(&this.0)
  }

  pub(crate) fn weak_count(this: &Ptr<T>) -> usize {
    Rc::weak_count(&this.0)
  }
}

impl Ptr<()> {
  pub(crate) unsafe fn increment_strong_count(addr: usize) {
    let ptr = addr as *const ();
    unsafe { Rc::increment_strong_count(ptr) }
  }

  pub(crate) unsafe fn decrement_strong_count(addr: usize) {
    let ptr = addr as *const ();
    unsafe { Rc::decrement_strong_count(ptr) }
  }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.0.as_ref())
  }
}

impl<T: std::fmt::Display> std::fmt::Display for Ptr<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.0, f)
  }
}
