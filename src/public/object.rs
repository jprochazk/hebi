pub mod function;
pub mod list;
pub mod string;
pub mod table;

use crate::object::{Any, Ptr};
use crate::Bind;

decl_object_ref! {
  struct Any
}

impl<'cx> AnyRef<'cx> {
  pub fn cast<T: ObjectRef<'cx>>(self) -> Option<T> {
    T::from_any(self)
  }
}

pub trait ObjectRef<'cx>: private::Sealed + Sized {
  fn as_any(&self) -> AnyRef<'cx>;
  fn from_any(v: AnyRef<'cx>) -> Option<Self>;

  // TODO: add same methods as `Object` and delegate
}

mod private {
  pub trait Sealed {}
}
