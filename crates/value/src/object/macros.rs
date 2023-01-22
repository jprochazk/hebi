// TODO: document panics due to unique xor shared borrow

macro_rules! object_repr {
  (
    enum $Repr:ident {
      $($ty:ty),*
      $(,)?
    }
  ) => {
    paste! {
      #[derive(Clone)]
      enum $Repr {
        $( $ty($ty), )*
      }

      impl Object {
        $(
          pub fn [<$ty:lower>](v: impl Into<$ty>) -> Self {
            Object { repr: $Repr::$ty(v.into()) }
          }

          pub fn [<is_ $ty:lower>](&self) -> bool {
            matches!(self.repr, $Repr::$ty(..))
          }

          pub fn [<as_ $ty:lower>](&self) -> Option<&$ty> {
            if let $Repr::$ty(ref v) = self.repr {
              Some(v)
            } else {
              None
            }
          }

          pub fn [<as_ $ty:lower _mut>](&mut self) -> Option<&mut $ty> {
            if let $Repr::$ty(ref mut v) = self.repr {
              Some(v)
            } else {
              None
            }
          }
        )*
      }

      impl Value {
        $(
          pub fn [<is_ $ty:lower>](&self) -> bool {
            let Some(this) = self.as_object() else { return false; };
            this.[<is_ $ty:lower>]()
          }

          pub fn [<as_ $ty:lower>](&self) -> Option<Ref<'_, $ty>> {
            let Some(this) = self.as_object() else { return None; };
            if !this.[<is_ $ty:lower>]() {
              return None;
            }
            Some(Ref::map(this, |v| v.[<as_ $ty:lower>]().unwrap()))
          }


          pub fn [<as_ $ty:lower _mut>](&mut self) -> Option<RefMut<'_, $ty>> {
            let Some(this) = self.as_object_mut() else { return None; };
            if !this.[<is_ $ty:lower>]() {
              return None;
            }
            Some(RefMut::map(this, |v| v.[<as_ $ty:lower _mut>]().unwrap()))
          }
        )*
      }

      $(
        impl From<$ty> for Object {
          fn from(v: $ty) -> Self {
            Object::[<$ty:lower>](v)
          }
        }
      )*
    }
  }
}
