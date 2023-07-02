use super::*;
use crate::internal::object::{Function as OwnedFunction, Ptr};

decl_ref! {
  struct Function(Ptr<OwnedFunction>)
}

impl_object_ref!(Function, OwnedFunction);

impl<'cx> Function<'cx> {}
