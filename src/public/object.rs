pub mod function;
pub mod list;
pub mod string;
pub mod table;

use std::fmt::{Debug, Display};

use crate::object::{Any, Ptr};
use crate::{Bind, Global};

decl_ref! {
  struct Any(Ptr<Any>)
}

impl<'cx> AnyRef<'cx> {
  pub fn cast<T: ObjectRef<'cx>>(self, global: Global<'cx>) -> Option<T> {
    T::from_any(self, global)
  }
}

impl<'cx> ObjectRef<'cx> for AnyRef<'cx> {
  fn as_any(&self, _: Global<'cx>) -> AnyRef<'cx> {
    let ptr = self.inner.clone().into_any();
    unsafe { ptr.bind_raw::<'cx>() }
  }

  fn from_any(v: AnyRef<'cx>, _: Global<'cx>) -> Option<Self> {
    Some(v)
  }
}

impl<'cx> private::Sealed for AnyRef<'cx> {}

pub trait ObjectRef<'cx>: private::Sealed + Sized + Debug + Display {
  fn as_any(&self, global: Global<'cx>) -> AnyRef<'cx>;
  fn from_any(v: AnyRef<'cx>, global: Global<'cx>) -> Option<Self>;

  // TODO: add same methods as `Object` and delegate
}

mod private {
  pub trait Sealed {}
}
