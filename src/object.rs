macro_rules! declare_object_trait {
  (trait $Object:ident -> $VTable:ident, $generate_vtable:ident {
    $(fn $name:ident($this:ident: Ptr<Self>, $($arg:ident : $ty:ty),*) -> $ret:ty $body:block)*
  }) => {
    #[repr(C)]
    pub struct $VTable<T: Sized + 'static> {
      pub(crate) drop_in_place: unsafe fn(*mut T),
      pub(crate) display_fmt: fn(*const T, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
      pub(crate) debug_fmt: fn(*const T, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result,

      $(pub(crate) $name : fn($this: Ptr<T>, $($arg:$ty),*) -> $ret),*
    }

    pub trait $Object: Debug + Display + Sized + 'static {
      $(
        #[allow(unused_variables)]
        fn $name($this: Ptr<Self>, $($arg:$ty),*) -> $ret $body
      )*
    }

    impl<T: $Object + Sized + 'static> $crate::object::ptr::Ptr<T> {
      $(
        pub fn $name(&self, $($arg:$ty),*) -> $ret {
          <T as $Object>::$name(self.clone(), $($arg),*)
        }
      )*
    }

    impl $Object for $crate::object::ptr::Any {
      $(
        fn $name(this: Ptr<$crate::object::ptr::Any>, $($arg:$ty),*) -> $ret {
          let method = unsafe { this.vtable() }.$name;
          let this = unsafe {
            ::core::mem::transmute::<Ptr<$crate::object::ptr::Any>, Ptr<()>>(this)
          };
          (method)(this, $($arg),*)
        }
      )*
    }

    macro_rules! $generate_vtable {
      ($T:ident) => {
        impl $crate::object::Type for $T {
          fn vtable() -> &'static $crate::object::VTable<Self> {
            static VTABLE: $crate::object::VTable<$T> = $crate::object::VTable {
              drop_in_place: ::std::ptr::drop_in_place::<$T>,
              display_fmt: |ptr, f| <$T as ::std::fmt::Display>::fmt(unsafe { &*ptr }, f),
              debug_fmt: |ptr, f| <$T as ::std::fmt::Debug>::fmt(unsafe { &*ptr }, f),

              $($name: <$T as $crate::object::Object>::$name),*
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

declare_object_trait! {
  trait Object -> VTable, generate_vtable {

    fn type_name(this: Ptr<Self>,) -> &'static str {
      "Unknown"
    }

    fn named_field(
      this: Ptr<Self>,
      scope: Scope<'_>,
      name: Ptr<Str>
    ) -> Result<Option<Value>> {
      let _ = scope;
      fail!("cannot get field `{name}`")
    }

    fn set_named_field(
      this: Ptr<Self>,
      scope: Scope<'_>,
      name: Ptr<Str>,
      value: Value
    ) -> Result<()> {
      let _ = scope;
      let _ = value;
      fail!("cannot set field `{name}`")
    }

    fn keyed_field(this: Ptr<Self>, scope: Scope<'_>, key: Value) -> Result<Option<Value>> {
      let _ = scope;
      fail!("`{this}` does not support `[]`")
    }

    fn set_keyed_field(
      this: Ptr<Self>,
      scope: Scope<'_>,
      key: Value,
      value: Value
    ) -> Result<()> {
      let _ = scope;
      let _ = value;
      fail!("`{this}` does not support `[]=`")
    }

    fn contains(this: Ptr<Self>, scope: Scope<'_>, item: Value) -> Result<bool> {
      let _ = scope;
      let _ = item;
      fail!("`{this}` does not support `in`")
    }

    fn add(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Value> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support `+`")
    }

    fn subtract(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Value> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support `-`")
    }

    fn multiply(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Value> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support `*`")
    }

    fn divide(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Value> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support `/`")
    }

    fn remainder(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Value> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support `%`")
    }

    fn pow(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Value> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support `**`")
    }

    fn invert(this: Ptr<Self>, scope: Scope<'_>) -> Result<Value> {
      let _ = scope;
      fail!("`{this}` does not support unary `-`")
    }

    fn not(this: Ptr<Self>, scope: Scope<'_>) -> Result<Value> {
      let _ = scope;
      fail!("`{this}` does not support `!`")
    }

    fn cmp(this: Ptr<Self>, scope: Scope<'_>, other: Value) -> Result<Ordering> {
      let _ = scope;
      let _ = other;
      fail!("`{this}` does not support comparison")
    }
  }
}

pub fn is_callable(v: &Ptr<Any>) -> bool {
  v.is::<Function>()
    || v.is::<ClassMethod>()
    || v.is::<NativeFunction>()
    || v.is::<NativeAsyncFunction>()
}

pub fn is_class(v: &Ptr<Any>) -> bool {
  v.is::<ClassInstance>() || v.is::<ClassProxy>() || v.is::<NativeClassInstance>()
}

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
pub use function::{Function, FunctionDescriptor};
pub use list::List;
pub use module::{Module, ModuleDescriptor};
pub use ptr::{Any, Ptr};
pub use string::Str;
pub use table::Table;

use self::class::{ClassInstance, ClassMethod, ClassProxy};
use self::native::{NativeAsyncFunction, NativeClassInstance, NativeFunction};
use crate::value::Value;
use crate::{Result, Scope};
