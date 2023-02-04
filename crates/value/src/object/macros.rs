// TODO: document panics due to unique xor shared borrow

macro_rules! object_repr {
  (
    enum $Repr:ident {
      $($ty:ty),*
      $(,)?
    }
  ) => {
    paste! {
      #[derive(Clone, Debug)]
      enum $Repr {
        $( $ty($ty), )*
      }

      impl Object {
        $(
          pub fn [<$ty:snake>](v: impl Into<$ty>) -> Self {
            Object { repr: $Repr::$ty(v.into()) }
          }

          pub fn [<is_ $ty:snake>](&self) -> bool {
            matches!(self.repr, $Repr::$ty(..))
          }

          pub fn [<as_ $ty:snake>](&self) -> Option<&$ty> {
            if let $Repr::$ty(ref v) = self.repr {
              Some(v)
            } else {
              None
            }
          }

          pub fn [<as_ $ty:snake _mut>](&mut self) -> Option<&mut $ty> {
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
          pub fn [<is_ $ty:snake>](&self) -> bool {
            let Some(this) = self.as_object() else { return false; };
            this.[<is_ $ty:snake>]()
          }

          pub fn [<as_ $ty:snake>](&self) -> Option<Ref<'_, $ty>> {
            let Some(this) = self.as_object() else { return None; };
            if !this.[<is_ $ty:snake>]() {
              return None;
            }
            Some(Ref::map(this, |v| v.[<as_ $ty:snake>]().unwrap()))
          }


          pub fn [<as_ $ty:snake _mut>](&mut self) -> Option<RefMut<'_, $ty>> {
            let Some(this) = self.as_object_mut() else { return None; };
            if !this.[<is_ $ty:snake>]() {
              return None;
            }
            Some(RefMut::map(this, |v| v.[<as_ $ty:snake _mut>]().unwrap()))
          }
        )*
      }

      $(
        impl From<$ty> for Object {
          fn from(v: $ty) -> Self {
            Object::[<$ty:snake>](v)
          }
        }


        impl private::Sealed for $ty {}
        impl ObjectHandle for $ty {
          fn as_self(o: &Ptr<Object>) -> Option<Ref<Self>> {
            if !o.borrow().[<is_ $ty:snake>]() {
              return None;
            }
            Some(Ref::map(o.borrow(), |v| v.[<as_ $ty:snake>]().unwrap()))
          }
          fn as_self_mut(o: &mut Ptr<Object>) -> Option<RefMut<Self>> {
            if !o.borrow().[<is_ $ty:snake>]() {
              return None;
            }
            Some(RefMut::map(o.borrow_mut(), |v| v.[<as_ $ty:snake _mut>]().unwrap()))
          }
          fn is_self(o: &Ptr<Object>) -> bool {
            o.borrow().[<is_ $ty:snake>]()
          }
        }
      )*
    }
  }
}
