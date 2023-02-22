macro_rules! object_repr {
  (
    enum $Repr:ident {
      $($ty:ty),*
      $(,)?
    }
  ) => {
    paste::paste! {
      enum $Repr {
        $( $ty($ty), )*
      }

      impl Object {
        $(
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

      impl Display for Object {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match &self.repr {
            $($Repr::$ty(inner) => Display::fmt(inner, f),)*
          }
        }
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

        fn field_get<'a>(&self, key: &Key<'a>) -> Result<Option<Value>, crate::RuntimeError> {
          match &self.repr {
            $($Repr::$ty(inner) => inner.field_get(key),)*
          }
        }

        fn field_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::RuntimeError> {
          match &mut self.repr {
            $($Repr::$ty(inner) => inner.field_set(key, value),)*
          }
        }

        fn index_get<'a>(&self, key: &Key<'a>) -> Result<Option<Value>, crate::RuntimeError> {
          match &self.repr {
            $($Repr::$ty(inner) => inner.index_get(key),)*
          }
        }

        fn index_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::RuntimeError> {
          match &mut self.repr {
            $($Repr::$ty(inner) => inner.index_set(key, value),)*
          }
        }
      }

      impl Value {
        $(
          pub fn [<is_ $ty:snake>](&self) -> bool {
            let Some(this) = self.as_object_raw() else { return false; };
            this.[<is_ $ty:snake>]()
          }

          pub fn [<to_ $ty:snake>](self) -> Option<Handle<$ty>> {
            self.to_object()
          }
        )*
      }

      $(
        impl From<$ty> for Object {
          fn from(v: $ty) -> Self {
            Object { repr: $Repr::$ty(v.into()) }
          }
        }

        impl private::Sealed for $ty {}
        impl ObjectType for $ty {
          fn as_ref(o: &Object) -> Option<&Self> {
            if !o.[<is_ $ty:snake>]() {
              return None;
            }
            Some(o.[<as_ $ty:snake>]().unwrap())
          }
          fn as_mut(o: &mut Object) -> Option<&mut Self> {
            if !o.[<is_ $ty:snake>]() {
              return None;
            }
            Some(o.[<as_ $ty:snake _mut>]().unwrap())
          }
          fn is(o: &Object) -> bool {
            o.[<is_ $ty:snake>]()
          }
        }
      )*
    }
  }
}
