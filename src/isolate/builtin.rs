use super::ClassMap;
use crate::ctx::Context;
use crate::value::object;
use crate::value::object::native::TypeInfo;
use crate::value::object::NativeClass;

pub fn register_builtins(ctx: &Context, class_map: &mut ClassMap, globals: &mut object::Dict) {
  macro_rules! register {
    ($class_map:ident {$($T:ty),*}) => {
      $(
        {
          let class = NativeClass::new::<$T>(ctx);
          $class_map.insert(
            ::std::any::TypeId::of::<$T>(),
            class.clone(),
          );
          globals.insert(
            ctx.alloc(object::Str::from(<$T as TypeInfo>::name())),
            class
          );
        }
      )*
    }
  }

  register! {
    class_map {
      object::List,
      object::Str,
      object::Dict
    }
  }
}
