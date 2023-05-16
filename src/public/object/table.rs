use super::*;
use crate::object::Table;

decl_object_ref! {
  struct Table
}

impl<'cx> TableRef<'cx> {}
