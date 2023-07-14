use core::fmt::{Debug, Display};

pub struct DelegateDebugToDisplay<T: Display>(pub T);

impl<T: Display> Debug for DelegateDebugToDisplay<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Display::fmt(&self.0, f)
  }
}
