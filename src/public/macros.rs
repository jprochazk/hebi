macro_rules! decl_ref {
  (struct $T:ident) => {
    paste::paste! {
      decl_ref!(__final [<$T Ref>], $T);
    }
  };
  (struct $name:ident($inner:ty)) => {
    paste::paste! {
      decl_ref!(__final [<$name Ref>], $inner);
    }
  };
  (__final $name:ident, $inner:ty) => {
    #[derive(Clone)]
    #[repr(C)]
    pub struct $name<'cx> {
      inner: $inner,
      lifetime: ::core::marker::PhantomData<&'cx ()>,
    }

    impl<'cx> ::std::fmt::Debug for $name<'cx> {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(&self.inner, f)
      }
    }

    impl<'cx> ::std::fmt::Display for $name<'cx> {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self.inner, f)
      }
    }

    unsafe impl<'cx> $crate::IsSimpleRef for $name<'cx> {}

    impl $crate::public::Bind for $inner {
      type Ref<'cx> = $name<'cx>;

      unsafe fn bind_raw<'cx>(self) -> Self::Ref<'cx> {
        ::std::mem::transmute::<Self, Self::Ref<'cx>>(self)
      }
    }

    impl<'cx> $crate::public::Unbind for $name<'cx> {
      type Owned = $inner;

      fn unbind(self) -> Self::Owned {
        unsafe { ::std::mem::transmute::<Self, Self::Owned>(self) }
      }
    }
  };
}

macro_rules! decl_object_ref {
  (struct $T:ident) => {
    decl_ref! {
      struct $T(Ptr<$T>)
    }

    paste::paste! {
      impl<'cx> $crate::public::object::ObjectRef<'cx> for [<$T Ref>]<'cx> {
        fn as_any(&self) -> $crate::public::object::AnyRef<'cx> {
          let ptr = self.inner.clone().into_any();
          unsafe { ptr.bind_raw::<'cx>() }
        }

        fn from_any(v: crate::public::object::AnyRef<'cx>) -> Option<Self> {
          v.inner.cast::<$T>().ok().map(|v| unsafe { v.bind_raw::<'cx>() })
        }
      }


      impl<'cx> $crate::public::object::private::Sealed for [<$T Ref>]<'cx> {}
    }
  };
}
