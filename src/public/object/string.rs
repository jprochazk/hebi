use super::*;
use crate::object::String;
use crate::Scope;

decl_object_ref! {
  struct String
}

impl<'cx> StringRef<'cx> {
  pub fn as_str(&self) -> &str {
    self.inner.as_str()
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_string(&self, v: impl ToString) -> StringRef<'cx> {
    self.cx().inner.alloc(String::owned(v)).bind(self.cx())
  }
}
