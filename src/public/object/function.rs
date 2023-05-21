use super::*;
use crate::object::{Function as OwnedFunction, Ptr};
use crate::Unbind;

decl_ref! {
  struct Function(Ptr<OwnedFunction>)
}

impl_object_ref!(Function, OwnedFunction);

impl<'cx> Function<'cx> {}
