use core::fmt::{Debug, Display};
use core::mem::size_of;

pub struct DelegateDebugToDisplay<T: Display>(pub T);

impl<T: Display> Debug for DelegateDebugToDisplay<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Display::fmt(&self.0, f)
  }
}

#[track_caller]
#[inline(never)]
#[cold]
pub const fn static_assert_size<T: Sized>(bytes: usize, msg: &'static str) {
  if size_of::<T>() != bytes {
    panic!("{}", msg)
  }
}

#[track_caller]
#[inline(never)]
#[cold]
pub const fn static_assert(v: bool, msg: &'static str) {
  if !v {
    panic!("{}", msg);
  }
}
