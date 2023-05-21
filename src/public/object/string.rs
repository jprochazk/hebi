use super::*;
use crate::object::Str;
use crate::Scope;

decl_object_ref! {
  struct Str
}

impl<'cx> StrRef<'cx> {
  pub fn as_str(&self) -> &str {
    self.inner.as_str()
  }
}

impl<'cx> Global<'cx> {
  pub fn new_string(&self, v: impl ToString) -> StrRef<'cx> {
    self.inner.alloc(Str::owned(v)).bind(self.clone())
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_string(&self, v: impl ToString) -> StrRef<'cx> {
    self.global().new_string(v)
  }
}
