use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::ops::Deref;
use core::ptr::NonNull;

use bumpalo::AllocErr;

use crate::gc::{Gc, Object, Ref};

pub struct Str {
  data: NonNull<str>,
}

impl Str {
  pub fn try_new_in(gc: &Gc, data: &str) -> Result<Ref<Self>, AllocErr> {
    let data = NonNull::from(gc.try_alloc_str(data)?);
    gc.try_alloc(Str { data })
  }

  pub fn try_intern_in(gc: &Gc, data: &str) -> Result<Ref<Self>, AllocErr> {
    let data = NonNull::from(gc.try_intern_str(data)?);
    gc.try_alloc(Str { data })
  }

  pub fn as_str(&self) -> &str {
    unsafe { self.data.as_ref() }
  }
}

impl Object for Str {}

impl Debug for Str {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Str").field("data", &self.as_str()).finish()
  }
}

impl Display for Str {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Display::fmt(self.as_str(), f)
  }
}

impl Deref for Str {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    self.as_str()
  }
}

impl Hash for Str {
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    self.as_str().hash(state)
  }
}

impl PartialEq for Str {
  fn eq(&self, other: &Self) -> bool {
    self.as_str() == other.as_str()
  }
}
impl Eq for Str {}

impl PartialEq<str> for Str {
  fn eq(&self, other: &str) -> bool {
    self.as_str().eq(other)
  }
}

impl PartialOrd for Str {
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    Some(self.cmp(other))
  }
}
impl Ord for Str {
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.as_str().cmp(other.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn alloc_str() {
    let gc = Gc::new();

    let v = Str::try_new_in(&gc, "test").unwrap();
    assert_eq!(v.as_str(), "test");
  }
}
