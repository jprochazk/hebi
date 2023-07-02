use super::*;
use crate::internal::object::{Ptr, Str as OwnedStr};
use crate::public::{Hebi, Scope};

decl_ref! {
  struct Str(Ptr<OwnedStr>)
}

impl_object_ref!(Str, OwnedStr);

impl<'cx> Str<'cx> {
  pub fn as_str(&self) -> &str {
    self.inner.as_str()
  }
}

impl<'cx> Global<'cx> {
  pub fn new_string(&self, v: impl ToString) -> Str<'cx> {
    self.inner.alloc(OwnedStr::owned(v)).bind(self.clone())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_string(&self, v: impl ToString) -> Str<'cx> {
    self.global().new_string(v)
  }
}

impl Hebi {
  pub fn new_string(&self, v: impl ToString) -> Str {
    self.global().new_string(v)
  }
}
