use super::Context;
use crate::value::{object, Value as CoreValue};
use crate::{Hebi, Result, RuntimeError, Value};

pub trait FromHebi<'a>: Sized + private::Sealed {
  fn from_hebi(ctx: &Context<'a>, value: Value<'a>) -> Result<Self>;
}

pub trait IntoHebi<'a>: Sized + private::Sealed {
  fn into_hebi(self, ctx: &Context<'a>) -> Result<Value<'a>>;
}

macro_rules! impl_int {
  ($($T:ident),*) => {
    $(
      impl private::Sealed for $T {}
      impl<'a> FromHebi<'a> for $T {
        fn from_hebi(_: &Context<'a>, value: Value<'a>) -> Result<Self> {
          let value = value
            .inner
            .to_int()
            .ok_or_else(|| RuntimeError::script("value is not an int", 0..0))?;
          Ok(value as $T)
        }
      }
      impl<'a> IntoHebi<'a> for $T {
        fn into_hebi(self, _: &Context<'a>) -> Result<Value<'a>> {
          let value = self as i32;
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
      impl<'a> FromHebi<'a> for $T {
        fn from_hebi(_: &Context<'a>, value: Value<'a>) -> Result<Self> {
          let value = value
            .inner
            .to_float()
            .ok_or_else(|| RuntimeError::script("value is not a float", 0..0))?;
          Ok(value as $T)
        }
      }
      impl<'a> IntoHebi<'a> for $T {
        fn into_hebi(self, _: &Context<'a>) -> Result<Value<'a>> {
          let value = self as f64;
          Ok(Value::bind(value))
        }
      }
    )*
  }
}

impl_float!(f32, f64);

impl private::Sealed for bool {}
impl<'a> FromHebi<'a> for bool {
  fn from_hebi(_: &Context<'a>, value: Value<'a>) -> Result<Self> {
    let value = value
      .inner
      .to_bool()
      .ok_or_else(|| RuntimeError::script("value is not a bool", 0..0))?;
    Ok(value)
  }
}
impl<'a> IntoHebi<'a> for bool {
  fn into_hebi(self, _: &Context<'a>) -> Result<Value<'a>> {
    Ok(Value::bind(self))
  }
}

impl private::Sealed for String {}
impl<'a> FromHebi<'a> for String {
  fn from_hebi(_: &Context<'a>, value: Value<'a>) -> Result<Self> {
    let value = value
      .inner
      .to_str()
      .map(|str| str.as_str().to_string())
      .ok_or_else(|| RuntimeError::script("value is not a string", 0..0))?;
    Ok(value)
  }
}
impl<'a> IntoHebi<'a> for String {
  fn into_hebi(self, ctx: &Context<'a>) -> Result<Value<'a>> {
    Ok(Value::bind(
      ctx.inner().alloc(crate::value::object::Str::from(self)),
    ))
  }
}

impl<'a> private::Sealed for &'a str {}
impl<'a, 'b> IntoHebi<'a> for &'b str {
  fn into_hebi(self, ctx: &Context<'a>) -> Result<Value<'a>> {
    Ok(Value::bind(ctx.inner().alloc(object::Str::from(self))))
  }
}

impl private::Sealed for () {}
impl<'a> FromHebi<'a> for () {
  fn from_hebi(_: &Context<'a>, _: Value<'a>) -> Result<Self> {
    Ok(())
  }
}
impl<'a> IntoHebi<'a> for () {
  fn into_hebi(self, _: &Context<'a>) -> Result<Value<'a>> {
    Ok(Value::bind(CoreValue::none()))
  }
}

impl<'a> private::Sealed for Value<'a> {}
impl<'a> FromHebi<'a> for Value<'a> {
  fn from_hebi(_: &Context<'a>, value: Value<'a>) -> Result<Self> {
    Ok(value.clone())
  }
}
impl<'a> IntoHebi<'a> for Value<'a> {
  fn into_hebi(self, _: &Context<'a>) -> Result<Value<'a>> {
    Ok(self)
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
      out.push(T::from_hebi(item.clone(), ctx)?);
    }
    Ok(out)
  }
  into(self, ctx) {
    let mut list = List::with_capacity(self.len());
    for item in self.into_iter() {
      list.push(item.to_hebi(ctx)?);
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
        K::from_hebi(k.clone().to_value(ctx), ctx)?,
        V::from_hebi(v.clone(), ctx)?
      );
    }
    Ok(out)
  }
  into(self, ctx) {
    let mut dict = Dict::with_capacity(self.len());
    for (k, v) in self.into_iter() {
      dict.insert(
        Key::try_from(k.to_hebi(ctx)?).map_err(|e| Error::new(format!("{e}"), 0..0))?,
        v.to_hebi(ctx)?
      );
    }
    Ok(ctx.alloc(dict).into())
  }
} */

mod private {
  pub trait Sealed {}
}
