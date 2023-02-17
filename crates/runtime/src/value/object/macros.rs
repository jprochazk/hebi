macro_rules! object_repr {
  (
    enum $Repr:ident {
      $($ty:ty),*
      $(,)?
    }
  ) => {
    paste::paste! {
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

      impl Access for Object {
        fn is_frozen(&self) -> bool {
          match &self.repr {
            $($Repr::$ty(inner) => inner.is_frozen(),)*
          }
        }

        fn should_bind_methods(&self) -> bool {
          match &self.repr {
            $($Repr::$ty(inner) => inner.should_bind_methods(),)*
          }
        }

        fn field_get<'a>(&self, key: &Key<'a>) -> Result<Option<Value>, crate::Error> {
          match &self.repr {
            $($Repr::$ty(inner) => inner.field_get(key),)*
          }
        }

        fn field_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::Error> {
          match &mut self.repr {
            $($Repr::$ty(inner) => inner.field_set(key, value),)*
          }
        }

        fn index_get<'a>(&self, key: &Key<'a>) -> Result<Option<Value>, crate::Error> {
          match &self.repr {
            $($Repr::$ty(inner) => inner.index_get(key),)*
          }
        }

        fn index_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::Error> {
          match &mut self.repr {
            $($Repr::$ty(inner) => inner.index_set(key, value),)*
          }
        }
      }

      impl Value {
        $(
          pub fn [<is_ $ty:snake>](&self) -> bool {
            let Some(this) = self.as_object() else { return false; };
            this.[<is_ $ty:snake>]()
          }

          pub fn [<as_ $ty:snake>](&self) -> Option<&$ty> {
            let Some(this) = self.as_object() else { return None; };
            if !this.[<is_ $ty:snake>]() {
              return None;
            }
            Some(this.[<as_ $ty:snake>]().unwrap())
          }

          pub fn [<as_ $ty:snake _mut>](&mut self) -> Option<&mut $ty> {
            let Some(this) = self.as_object_mut() else { return None; };
            if !this.[<is_ $ty:snake>]() {
              return None;
            }
            Some(this.[<as_ $ty:snake _mut>]().unwrap())
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
          fn as_self(o: &Ptr<Object>) -> Option<&Self> {
            if !o.get().[<is_ $ty:snake>]() {
              return None;
            }
            Some(o.get().[<as_ $ty:snake>]().unwrap())
          }
          fn as_self_mut(o: &mut Ptr<Object>) -> Option<&mut Self> {
            if !o.get_mut().[<is_ $ty:snake>]() {
              return None;
            }
            Some(o.get_mut().[<as_ $ty:snake _mut>]().unwrap())
          }
          fn is_self(o: &Ptr<Object>) -> bool {
            o.get().[<is_ $ty:snake>]()
          }
        }
      )*
    }
  }
}
