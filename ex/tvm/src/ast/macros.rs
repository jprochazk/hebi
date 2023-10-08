macro_rules! decl {
  (
    $name:ident<$lifetime:lifetime> {
      $(
        $variant:ident $({
          $($field:ident : $ty:ty),* $(,)?
        })?
      ),* $(,)?
    }
  ) => {
    paste::paste! {
      #[derive(Clone, Debug)]
      pub struct $name<$lifetime> {
        pub kind: [<$name Kind>]<$lifetime>,
        pub span: Span,
      }

      impl<$lifetime> $name<$lifetime> {
        $(
          pub fn [<make_ $variant:snake>](
            span: impl Into<crate::lex::Span>,
            $($($field : $ty),*)?
          ) -> Self {
            Self {
              kind: [<$name Kind>]::$variant(
                [<$variant $name>]::new(
                  $($($field),*)?
                ).wrap_box()
              ),
              span: span.into(),
            }
          }

          pub fn [<is_ $variant:snake>](&self) -> bool {
            matches!(self.kind, [<$name Kind>]::$variant(..))
          }

          #[allow(unused_parens)]
          pub fn [<into_ $variant:snake>](self) -> Option<([<$variant $name>]<$lifetime>)> {
            match self.kind {
              [<$name Kind>]::$variant(inner) => Some(inner.unwrap_box()),
              _ => None,
            }
          }

          #[allow(unused_parens)]
          pub fn [<as_ $variant:snake>](&self) -> Option<&([<$variant $name>]<$lifetime>)> {
            match &self.kind {
              [<$name Kind>]::$variant(inner) => Some(&*inner),
              _ => None,
            }
          }
        )*
      }

      #[derive(Clone, Debug)]
      pub enum [<$name Kind>]<$lifetime> {
        $(
          $variant(
            decl!(@maybe_box $variant $name $lifetime $({ $($field : $ty),* })?)
          )
        ),*
      }
    }

    $(
      decl!(@variant_struct $variant $name $lifetime $({ $($field : $ty),* })?);
    )*
  };

  (@maybe_box
    $variant:ident
    $name:ident
    $lifetime:lifetime
    { $($field:ident : $ty:ty),* }
  ) => {
    paste::paste! {
      Box<[<$variant $name>]<$lifetime>>
    }
  };
  (@maybe_box
    $variant:ident
    $name:ident
    $lifetime:lifetime
  ) => {
    paste::paste! {
      [<$variant $name>]<$lifetime>
    }
  };

  (@variant_struct
    $variant:ident
    $name:ident
    $lifetime:lifetime
    { $($field:ident : $ty:ty),* }
  ) => {
    paste::paste! {
      #[derive(Clone, Debug)]
      pub struct [<$variant $name>]<$lifetime> {
        _lifetime: std::marker::PhantomData<& $lifetime ()>,
        $(pub $field : $ty),*
      }

      impl<$lifetime> [<$variant $name>]<$lifetime> {
        pub fn new($($field : $ty),*) -> Self {
          Self {
            _lifetime: std::marker::PhantomData,
            $($field),*
          }
        }
      }

      impl<$lifetime> WrapBox for [<$variant $name>]<$lifetime> {
        type Boxed = Box<Self>;

        fn wrap_box(self) -> Self::Boxed {
          Box::new(self)
        }
      }

      impl<$lifetime> UnwrapBox for Box<[<$variant $name>]<$lifetime>> {
        type Unboxed = [<$variant $name>]<$lifetime>;

        fn unwrap_box(self) -> Self::Unboxed {
          *self
        }
      }
    }
  };

  (@variant_struct
    $variant:ident
    $name:ident
    $lifetime:lifetime
  ) => {
    paste::paste! {
      #[derive(Clone, Debug)]
      pub struct [<$variant $name>]<$lifetime>(std::marker::PhantomData<& $lifetime ()>);

      impl<$lifetime> [<$variant $name>]<$lifetime> {
        pub fn new() -> Self {
          Self(std::marker::PhantomData)
        }
      }

      impl<$lifetime> WrapBox for [<$variant $name>]<$lifetime> {
        type Boxed = Self;

        fn wrap_box(self) -> Self::Boxed {
          self
        }
      }

      impl<$lifetime> UnwrapBox for [<$variant $name>]<$lifetime> {
        type Unboxed = Self;

        fn unwrap_box(self) -> Self::Unboxed {
          self
        }
      }
    }
  };
}
