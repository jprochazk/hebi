use super::*;
use crate::object::String;

decl_object_ref! {
  struct String
}

impl<'cx> StringRef<'cx> {}
