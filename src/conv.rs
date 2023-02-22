use std::fmt::Display;
use std::marker::PhantomData;

use object::Error as RuntimeError;
use value::Value as CoreValue;

use crate::value::object;
use crate::{value, Mu, Result};

pub struct Value<'a> {
  inner: crate::value::Value,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> Value<'a> {
  pub fn bind(value: impl Into<CoreValue>) -> Value<'a> {
    Self {
      inner: value.into(),
      _lifetime: PhantomData,
    }
  }
}

impl<'a> Display for Value<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.inner, f)
  }
}

pub trait FromMu<'a>: Sized + private::Sealed {
  fn from_mu(vm: &'a Mu, value: Value<'a>) -> Result<Self>;
}

pub trait IntoMu<'a>: Sized + private::Sealed {
  fn into_mu(vm: &'a Mu, value: Self) -> Result<Value<'a>>;
}

macro_rules! impl_int {
  ($($T:ident),*) => {
    $(
      impl private::Sealed for $T {}
      impl<'a> FromMu<'a> for $T {
        fn from_mu(_: &'a Mu, value: Value<'a>) -> Result<Self> {
          let value = value
            .inner
            .to_int()
            .ok_or_else(|| RuntimeError::new("value is not an int", 0..0))?;
          Ok(value as $T)
        }
      }
      impl<'a> IntoMu<'a> for $T {
        fn into_mu(_: &'a Mu, value: Self) -> Result<Value<'a>> {
          let value = value as i32;
          Ok(Value::bind(value))
        }
      }
    )*
  };
}

impl_int!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

macro_rules! impl_float {
  ($($T:ident),*) => {
    $(
      impl private::Sealed for $T {}
      impl<'a> FromMu<'a> for $T {
        fn from_mu(_: &'a Mu, value: Value<'a>) -> Result<Self> {
          let value = value
            .inner
            .to_float()
            .ok_or_else(|| RuntimeError::new("value is not a float", 0..0))?;
          Ok(value as $T)
        }
      }
      impl<'a> IntoMu<'a> for $T {
        fn into_mu(_: &'a Mu, value: Self) -> Result<Value<'a>> {
          let value = value as f64;
          Ok(Value::bind(value))
        }
      }
    )*
  }
}

impl_float!(f32, f64);

impl private::Sealed for bool {}
impl<'a> FromMu<'a> for bool {
  fn from_mu(_: &'a Mu, value: Value<'a>) -> Result<Self> {
    let value = value
      .inner
      .to_bool()
      .ok_or_else(|| RuntimeError::new("value is not a bool", 0..0))?;
    Ok(value)
  }
}
impl<'a> IntoMu<'a> for bool {
  fn into_mu(_: &'a Mu, value: Self) -> Result<Value<'a>> {
    Ok(Value::bind(value))
  }
}

impl private::Sealed for String {}
impl<'a> FromMu<'a> for String {
  fn from_mu(_: &'a Mu, value: Value<'a>) -> Result<Self> {
    let value = value
      .inner
      .to_str()
      .map(|str| str.as_str().to_string())
      .ok_or_else(|| RuntimeError::new("value is not a string", 0..0))?;
    Ok(value)
  }
}
impl<'a> IntoMu<'a> for String {
  fn into_mu(vm: &'a Mu, value: Self) -> Result<Value<'a>> {
    Ok(Value::bind(
      vm.isolate.borrow_mut().alloc(object::Str::from(value)),
    ))
  }
}

impl private::Sealed for () {}
impl<'a> FromMu<'a> for () {
  fn from_mu(_: &'a Mu, _: Value<'a>) -> Result<Self> {
    Ok(())
  }
}
impl<'a> IntoMu<'a> for () {
  fn into_mu(_: &'a Mu, _: Self) -> Result<Value<'a>> {
    Ok(Value::bind(CoreValue::none()))
  }
}

impl<'a> private::Sealed for Value<'a> {}
impl<'a> FromMu<'a> for Value<'a> {
  fn from_mu(_: &'a Mu, value: Value<'a>) -> Result<Self> {
    Ok(value)
  }
}
impl<'a> IntoMu<'a> for Value<'a> {
  fn into_mu(_: &'a Mu, value: Value<'a>) -> Result<Value<'a>> {
    Ok(value)
  }
}

/* conversion! {
  String
  from(value, _ctx) {
    value
      .to_str()
      .map(|str| str.as_str().to_string())
      .ok_or_else(|| Error::new("value is not a string", 0..0))
  }
  into(self, ctx) {
    Ok(ctx.alloc(Str::from(self)).into())
  }
}
conversion! {
  Vec<T>
  from(value, ctx) {
    let list = value.to_list().ok_or_else(|| Error::new("value is not a list", 0..0))?;
    let mut out = Vec::with_capacity(list.len());
    for item in list.iter() {
      out.push(T::from_mu(item.clone(), ctx)?);
    }
    Ok(out)
  }
  into(self, ctx) {
    let mut list = List::with_capacity(self.len());
    for item in self.into_iter() {
      list.push(item.to_mu(ctx)?);
    }
    Ok(ctx.alloc(list).into())
  }
}
conversion! {
  HashMap<K, V>
  where K: {Eq + Hash};
  from(value, ctx) {
    let dict = value.to_dict().ok_or_else(|| Error::new("value is not a dictionary", 0..0))?;
    let mut out = HashMap::with_capacity(dict.len());
    for (k, v) in dict.iter() {
      out.insert(
        K::from_mu(k.clone().to_value(ctx), ctx)?,
        V::from_mu(v.clone(), ctx)?
      );
    }
    Ok(out)
  }
  into(self, ctx) {
    let mut dict = Dict::with_capacity(self.len());
    for (k, v) in self.into_iter() {
      dict.insert(
        Key::try_from(k.to_mu(ctx)?).map_err(|e| Error::new(format!("{e}"), 0..0))?,
        v.to_mu(ctx)?
      );
    }
    Ok(ctx.alloc(dict).into())
  }
} */

mod private {
  pub trait Sealed {}
}
