pub mod function;
pub mod list;
pub mod string;
pub mod table;

use std::fmt::{Debug, Display};

use crate::object::{Any, Ptr};
use crate::{Bind, Global};

decl_object_ref! {
  struct Any
}

impl<'cx> AnyRef<'cx> {
  pub fn cast<T: ObjectRef<'cx>>(self, global: Global<'cx>) -> Option<T> {
    T::from_any(self, global)
  }
}

pub trait ObjectRef<'cx>: private::Sealed + Sized + Debug + Display {
  fn as_any(&self, global: Global<'cx>) -> AnyRef<'cx>;
  fn from_any(v: AnyRef<'cx>, global: Global<'cx>) -> Option<Self>;

  // TODO: add same methods as `Object` and delegate
}

mod private {
  pub trait Sealed {}
}
