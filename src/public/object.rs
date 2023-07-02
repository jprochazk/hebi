pub mod function;
pub mod list;
pub mod string;
pub mod table;

use std::fmt::{Debug, Display};

use crate::internal::object::{self, Ptr};
use crate::public::{Bind, Global};

decl_ref! {
  struct Any(Ptr<object::Any>)
}

impl<'cx> Any<'cx> {
  pub fn cast<T: ObjectRef<'cx>>(self, global: Global<'cx>) -> Option<T> {
    T::from_any(self, global)
  }
}

impl<'cx> ObjectRef<'cx> for Any<'cx> {
  fn as_any(&self, _: Global<'cx>) -> Any<'cx> {
    let ptr = self.inner.clone().into_any();
    unsafe { ptr.bind_raw::<'cx>() }
  }

  fn from_any(v: Any<'cx>, _: Global<'cx>) -> Option<Self> {
    Some(v)
  }
}

impl<'cx> private::Sealed for Any<'cx> {}

pub trait ObjectRef<'cx>: private::Sealed + Sized + Debug + Display {
  fn as_any(&self, global: Global<'cx>) -> Any<'cx>;
  fn from_any(v: Any<'cx>, global: Global<'cx>) -> Option<Self>;

  // TODO: add same methods as `Object` and delegate
}

mod private {
  pub trait Sealed {}
}
