use super::*;
use crate::object::Function;

decl_object_ref! {
  struct Function
}

impl<'cx> FunctionRef<'cx> {}
