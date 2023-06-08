macro_rules! decl_ref {
  (struct $name:ident($inner:ty)) => {
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

    unsafe impl<'cx> $crate::public::IsSimpleRef for $name<'cx> {}

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

macro_rules! impl_object_ref {
  ($T:ident, $Owned:ty) => {
    impl<'cx> $crate::public::object::ObjectRef<'cx> for $T<'cx> {
      fn as_any(&self, _: $crate::public::Global<'cx>) -> $crate::public::object::Any<'cx> {
        let ptr = self.inner.clone().into_any();
        unsafe { ptr.bind_raw::<'cx>() }
      }

      fn from_any(
        v: $crate::public::object::Any<'cx>,
        _: $crate::public::Global<'cx>,
      ) -> Option<Self> {
        v.inner
          .cast::<$Owned>()
          .ok()
          .map(|v| unsafe { v.bind_raw::<'cx>() })
      }
    }

    impl<'cx> $crate::public::object::private::Sealed for $T<'cx> {}
  };
}
