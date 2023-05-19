use super::object::{AnyRef, ObjectRef};
use crate::value::Value;
use crate::{Bind, Global, Result, Unbind};

decl_ref! {
  struct Value
}

impl<'cx> ValueRef<'cx> {
  pub fn as_float(&self) -> Option<f64> {
    self.inner.clone().to_float()
  }

  pub fn as_int(&self) -> Option<i32> {
    self.inner.clone().to_int()
  }

  pub fn as_bool(&self) -> Option<bool> {
    self.inner.clone().to_bool()
  }

  pub fn as_none(&self) -> Option<()> {
    self.inner.clone().to_none()
  }

  pub fn as_object<T: ObjectRef<'cx>>(&self, global: Global<'cx>) -> Option<T> {
    self.as_any().and_then(|v| AnyRef::cast(v, global))
  }

  pub fn as_any(&self) -> Option<AnyRef<'cx>> {
    self.inner.clone().to_any().map(|v| {
      // SAFETY: `self` is already bound to 'cx
      unsafe { v.bind_raw::<'cx>() }
    })
  }
}

pub trait FromValue<'cx>: Sized {
  fn from_value(value: ValueRef<'cx>, global: Global<'cx>) -> Result<Self>;
}

pub trait IntoValue<'cx>: Sized {
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>>;
}

impl<'cx> IntoValue<'cx> for ValueRef<'cx> {
  fn into_value(self, _: Global<'cx>) -> Result<ValueRef<'cx>> {
    Ok(self)
  }
}

impl<'cx> FromValue<'cx> for ValueRef<'cx> {
  fn from_value(value: ValueRef<'cx>, _: Global<'cx>) -> Result<Self> {
    Ok(value)
  }
}

impl<'cx> IntoValue<'cx> for i32 {
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    Ok(Value::int(self).bind(global))
  }
}

impl<'cx> FromValue<'cx> for i32 {
  fn from_value(value: ValueRef<'cx>, _: Global<'cx>) -> Result<Self> {
    match value.as_int() {
      Some(value) => Ok(value),
      None => crate::fail!("value is not an int"),
    }
  }
}

impl<'cx> IntoValue<'cx> for f64 {
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    Ok(Value::float(self).bind(global))
  }
}

impl<'cx> FromValue<'cx> for f64 {
  fn from_value(value: ValueRef<'cx>, _: Global<'cx>) -> Result<Self> {
    match value.as_float() {
      Some(value) => Ok(value),
      None => crate::fail!("value is not a float"),
    }
  }
}

impl<'cx> IntoValue<'cx> for bool {
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    Ok(Value::bool(self).bind(global))
  }
}

impl<'cx> FromValue<'cx> for bool {
  fn from_value(value: ValueRef<'cx>, _: Global<'cx>) -> Result<Self> {
    match value.as_bool() {
      Some(value) => Ok(value),
      None => crate::fail!("value is not a bool"),
    }
  }
}

impl<'cx> IntoValue<'cx> for () {
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    Ok(Value::none().bind(global))
  }
}

impl<'cx> FromValue<'cx> for () {
  fn from_value(value: ValueRef<'cx>, global: Global<'cx>) -> Result<Self> {
    let _ = (value, global);
    Ok(())
  }
}

impl<'cx, T> IntoValue<'cx> for Option<T>
where
  T: IntoValue<'cx>,
{
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    match self {
      Some(value) => value.into_value(global),
      None => Ok(Value::none().bind(global)),
    }
  }
}

impl<'cx, T> IntoValue<'cx> for Result<T>
where
  T: IntoValue<'cx>,
{
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    self?.into_value(global)
  }
}

impl<'cx, T> IntoValue<'cx> for T
where
  T: ObjectRef<'cx>,
{
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    Ok(Value::object(self.as_any(global.clone()).unbind()).bind(global))
  }
}

impl<'cx, T> FromValue<'cx> for T
where
  T: ObjectRef<'cx>,
{
  fn from_value(value: ValueRef<'cx>, global: Global<'cx>) -> Result<Self> {
    let object = value
      .as_any()
      .ok_or_else(|| error!("value is not an object"))?;
    let object = T::from_any(object, global).ok_or_else(|| {
      error!(
        "value is not an instance of {}",
        ::core::any::type_name::<T>()
      )
    })?;
    Ok(object)
  }
}

impl<'cx> FromValue<'cx> for String {
  fn from_value(value: ValueRef<'cx>, _: Global<'cx>) -> Result<Self> {
    let Some(str) = value.unbind().to_object::<crate::object::String>() else {
      fail!("value is not a string")
    };
    Ok(str.as_str().to_string())
  }
}

impl<'cx> IntoValue<'cx> for String {
  fn into_value(self, global: Global<'cx>) -> Result<ValueRef<'cx>> {
    global.new_string(self).into_value(global)
  }
}

pub trait FromValuePack<'cx> {
  type Output: Sized;
  fn from_value_pack(args: &[Value], global: Global<'cx>) -> Result<Self::Output>;
}

impl<'cx> FromValuePack<'cx> for () {
  type Output = ();

  fn from_value_pack(args: &[Value], _: Global<'cx>) -> Result<Self::Output> {
    #[allow(clippy::len_zero)]
    if args.len() > 0 {
      fail!("expected at most 0 args, got {}", args.len());
    }
    Ok(())
  }
}

macro_rules! impl_from_value_pack {
  ($($T:ident),*) => {
    impl<'cx, $($T),*> FromValuePack<'cx> for ($($T,)*)
    where
      $(
        $T: FromValue<'cx>,
      )*
    {
      type Output = ($($T,)*);

      #[allow(non_snake_case)]
      fn from_value_pack(args: &[Value], global: Global<'cx>) -> Result<Self::Output> {
        const NUM_ARGS: usize = __count!($($T)*);
        if args.len() > NUM_ARGS {
          fail!("expected at most {NUM_ARGS} args, got {}", args.len());
        }
        if args.len() < NUM_ARGS {
          fail!("expected at least {NUM_ARGS} args, got {}", args.len());
        }

        let mut offset = 0;
        $(
          let $T = unsafe { args.get_unchecked(offset).clone() }.bind(global.clone());
          let $T = <$T>::from_value($T, global.clone())?;
          offset += 1;
        )*
        let _ = offset;

        Ok(($($T,)*))
      }
    }
  };
}

impl_from_value_pack!(A);
impl_from_value_pack!(A, B);
impl_from_value_pack!(A, B, C);
impl_from_value_pack!(A, B, C, D);
impl_from_value_pack!(A, B, C, D, E);
impl_from_value_pack!(A, B, C, D, E, F);
impl_from_value_pack!(A, B, C, D, E, F, G);
impl_from_value_pack!(A, B, C, D, E, F, G, H);
impl_from_value_pack!(A, B, C, D, E, F, G, H, I);
impl_from_value_pack!(A, B, C, D, E, F, G, H, I, J);
impl_from_value_pack!(A, B, C, D, E, F, G, H, I, J, K);
impl_from_value_pack!(A, B, C, D, E, F, G, H, I, J, K, L);
