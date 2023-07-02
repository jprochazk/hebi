macro_rules! map_self_to {
  (Self, $($to:tt)*) => ($($to)*);
  ($other:ident, $($to:tt)*) => ($other);
}

macro_rules! transmute_self_arg {
  ($arg:ident Self) => {
    let $arg = unsafe { ::core::mem::transmute::<Ptr<Any>, Ptr<()>>($arg) };
  };
  ($arg:ident $ty:ident) => {};
}

macro_rules! declare_object_trait {
  (trait $Object:ident -> $VTable:ident, $declare_object_type:ident {
    $(fn $name:ident($scope:ident, $this:ident $(, $arg:ident : $ty:ident)*) -> $ret:ty $body:block)*
  }) => {
    declare_object_trait!(
      __final
      $Object $VTable $declare_object_type
      $( $name $scope $this $(($arg; $ty; map_self_to!($ty, Ptr<T>); map_self_to!($ty, Ptr<Self>); map_self_to!($ty, Ptr<T>); map_self_to!($ty, Ptr<Any>))),* -> $ret $body )*
    );
  };

  (
    __final
    $Object:ident $VTable:ident $declare_object_type:ident
    $(
      $name:ident $scope:ident $this:ident $(
        ($arg:ident; $ty:ident; $ty_in_vtable:ty; $ty_in_trait:ty; $ty_in_ptr:ty; $ty_in_any:ty)
      ),* -> $ret:ty $body:block
    )*
  ) => {
    #[repr(C)]
    pub struct $VTable<T: Sized + 'static> {
      pub(crate) drop_in_place: unsafe fn(*mut T),
      pub(crate) display_fmt: fn(*const T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
      pub(crate) debug_fmt: fn(*const T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,

      pub(crate) type_name: fn(Ptr<T>) -> &'static str,
      pub(crate) instance_of: fn(Ptr<T>, Value) -> Result<bool>,
      $(
        pub(crate) $name : fn(
          Scope<'_>,
          Ptr<T>,
          $($ty_in_vtable),*
        ) -> $ret
      ),*
    }

    pub trait $Object: Debug + Display + Sized + 'static {
      fn type_name(this: Ptr<Self>) -> &'static str;
      fn instance_of(this: Ptr<Self>, ty: Value) -> Result<bool>;
      $(
        fn $name(
          $scope: Scope<'_>,
          $this: Ptr<Self>,
          $($arg:$ty_in_trait),*
        ) -> $ret $body
      )*
    }

    impl<T: $Object + Sized + 'static> Ptr<T> {
      pub fn type_name(&self) -> &'static str {
        <T as $Object>::type_name(self.clone())
      }

      pub fn instance_of(&self, ty: Value) -> Result<bool> {
        <T as $Object>::instance_of(self.clone(), ty)
      }

      $(
        pub fn $name(
          &self,
          $scope: Scope<'_>,
          $($arg:$ty_in_ptr),*
        ) -> $ret {
          <T as $Object>::$name($scope, self.clone(), $($arg),*)
        }
      )*
    }

    impl $Object for Any {
      fn type_name(this: Ptr<Any>) -> &'static str {
        let method = unsafe { this.vtable() }.type_name;
        let this = unsafe {
          ::core::mem::transmute::<
            Ptr<Any>,
            Ptr<()>
          >(this)
        };
        (method)(this)
      }

      fn instance_of(this: Ptr<Any>, ty: Value) -> Result<bool> {
        let method = unsafe { this.vtable() }.instance_of;
        let this = unsafe {
          ::core::mem::transmute::<
            Ptr<Any>,
            Ptr<()>
          >(this)
        };
        (method)(this, ty)
      }

      $(
        fn $name(
          $scope: Scope<'_>,
          $this: Ptr<Any>,
          $($arg:$ty_in_any),*
        ) -> $ret {
          let method = unsafe { $this.vtable() }.$name;
          let this = unsafe {
            ::core::mem::transmute::<
              Ptr<Any>,
              Ptr<()>
            >($this)
          };
          $(
            transmute_self_arg!($arg $ty);
          )*
          (method)($scope, this, $($arg),*)
        }
      )*
    }

    macro_rules! $declare_object_type {
      ($T:ident) => {
        impl $crate::internal::object::Type for $T {
          fn vtable() -> &'static $crate::internal::object::VTable<Self> {
            static VTABLE: $crate::internal::object::VTable<$T> = $crate::internal::object::VTable {
              drop_in_place: ::std::ptr::drop_in_place::<$T>,
              display_fmt: |ptr, f| <$T as ::std::fmt::Display>::fmt(unsafe { &*ptr }, f),
              debug_fmt: |ptr, f| <$T as ::std::fmt::Debug>::fmt(unsafe { &*ptr }, f),

              type_name: <$T as $crate::internal::object::Object>::type_name,

              instance_of: <$T as $crate::internal::object::Object>::instance_of,
              $($name: <$T as $crate::internal::object::Object>::$name),*
            };

            &VTABLE
          }
        }
      }
    }
  };
}

pub trait Type: Sized + Object {
  fn vtable() -> &'static VTable<Self>;
}

type StrPtr = Ptr<Str>;

declare_object_trait! {
  trait Object -> VTable, declare_object_type {
    fn named_field(scope, this, name: StrPtr) -> Result<Value> {
      let _ = scope;
      let _ = name;
      let this = Self::type_name(this);
      fail!("`{this}` does not support field access")
    }

    fn named_field_opt(scope, this, name: StrPtr) -> Result<Option<Value>> {
      let _ = scope;
      let _ = name;
      let this = Self::type_name(this);
      fail!("`{this}` does not support field access")
    }

    fn set_named_field(scope, this, name: StrPtr, value: Value) -> Result<()> {
      let _ = scope;
      let _ = value;
      let _ = name;
      let this = Self::type_name(this);
      fail!("`{this}` does not support field access")
    }

    fn keyed_field(scope, this, key: Value) -> Result<Value> {
      let _ = scope;
      let _ = key;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `[]`")
    }

    fn keyed_field_opt(scope, this, key: Value) -> Result<Option<Value>> {
      let _ = scope;
      let _ = key;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `[]`")
    }

    fn set_keyed_field(scope, this, key: Value, value: Value) -> Result<()> {
      let _ = scope;
      let _ = key;
      let this = Self::type_name(this);
      let _ = value;
      fail!("`{this}` does not support `[]=`")
    }

    fn call(scope, this, return_addr: ReturnAddr) -> Result<CallResult> {
      let _ = scope;
      let _ = return_addr;
      let this = Self::type_name(this);
      fail!("`{this}` is not callable")
    }

    fn contains(scope, this, item: Value) -> Result<bool> {
      let _ = scope;
      let _ = item;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `in`")
    }

    fn add(scope, this, other: Self) -> Result<Value> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `+`")
    }

    fn subtract(scope, this, other: Self) -> Result<Value> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `-`")
    }

    fn multiply(scope, this, other: Self) -> Result<Value> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `*`")
    }

    fn divide(scope, this, other: Self) -> Result<Value> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `/`")
    }

    fn remainder(scope, this, other: Self) -> Result<Value> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `%`")
    }

    fn pow(scope, this, other: Self) -> Result<Value> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `**`")
    }

    fn invert(scope, this) -> Result<Value> {
      let _ = scope;
      let this = Self::type_name(this);
      fail!("`{this}` does not support unary `-`")
    }

    fn not(scope, this) -> Result<Value> {
      let _ = scope;
      let this = Self::type_name(this);
      fail!("`{this}` does not support `!`")
    }

    fn cmp(scope, this, other: Self) -> Result<Ordering> {
      let _ = scope;
      let _ = other;
      let this = Self::type_name(this);
      fail!("`{this}` does not support comparison")
    }
  }
}

macro_rules! default_instance_of {
  () => {
    fn instance_of(
      _: $crate::internal::object::ptr::Ptr<Self>,
      ty: $crate::internal::value::Value,
    ) -> $crate::internal::error::Result<bool> {
      Ok(ty.to_object::<Self>().is_some())
    }
  };
}

pub fn is_callable(v: &Ptr<Any>) -> bool {
  v.is::<Function>()
    || v.is::<BoundFunction>()
    || v.is::<NativeFunction>()
    || v.is::<NativeAsyncFunction>()
}

pub fn is_class(v: &Ptr<Any>) -> bool {
  v.is::<ClassInstance>() || v.is::<ClassProxy>() || v.is::<NativeClassInstance>()
}

pub type ReturnAddr = Option<usize>;

#[macro_use]
pub mod builtin;

pub mod class;
pub mod function;
pub mod list;
pub mod module;
pub mod native;
pub mod string;
pub mod table;

pub(crate) mod ptr;

use std::cmp::Ordering;
use std::fmt::{Debug, Display};

pub use class::{ClassDescriptor, ClassType};
pub use function::{BoundFunction, Function, FunctionDescriptor};
pub use list::List;
pub use module::{Module, ModuleDescriptor};
pub use ptr::{Any, Ptr};
pub use string::Str;
pub use table::Table;

use self::class::{ClassInstance, ClassProxy};
use self::native::{NativeAsyncFunction, NativeClassInstance, NativeFunction};
use super::error::Result;
use super::value::Value;
use super::vm::thread::CallResult;
use crate::public::Scope;
