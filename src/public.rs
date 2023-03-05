pub(crate) mod conv;

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use value::Value as CoreValue;

use crate::ctx::Context as CoreContext;
use crate::value;
use crate::value::object;

// TODO: make macro for wrapping types
// - `T` -> `T<'a> { inner: T, _lifetime: PhantomData<&'a ()> }`
// - T<'a>::bind(T) -> T<'a>
// - Value::as_T(self) -> Option<T<'a>>
// - From<T<'a>> for Value<'a>

// TODO: `Context` type for allocating objects
// TODO: convert all the `unsafe { transmute(v) }` into a safe interface with a
// SAFETY comment

#[repr(C)]
pub struct Context<'a> {
  inner: CoreContext,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> Context<'a> {
  pub(crate) fn inner(&self) -> &CoreContext {
    &self.inner
  }

  pub(crate) fn bind(inner: CoreContext) -> Self {
    Self {
      inner,
      _lifetime: PhantomData,
    }
  }

  pub(crate) fn unbind(self) -> CoreContext {
    self.inner
  }
}

#[repr(C)]
#[derive(Clone)]
pub struct Value<'a> {
  inner: CoreValue,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> Value<'a> {
  pub fn is_float(&self) -> bool {
    self.inner.is_float()
  }

  pub fn as_float(self) -> Option<f64> {
    self.inner.to_float()
  }

  pub fn is_int(&self) -> bool {
    self.inner.is_int()
  }

  pub fn as_int(self) -> Option<i32> {
    self.inner.to_int()
  }

  pub fn is_bool(&self) -> bool {
    self.inner.is_bool()
  }

  pub fn as_bool(self) -> Option<bool> {
    self.inner.to_bool()
  }

  pub fn is_none(&self) -> bool {
    self.inner.is_none()
  }

  pub fn as_none(self) -> Option<()> {
    self.inner.to_none()
  }

  pub fn is_str(&self) -> bool {
    self.inner.is_str()
  }

  pub fn as_str(self) -> Option<Str<'a>> {
    self.inner.to_str().map(Str::bind)
  }

  pub fn as_str_ref(&self) -> Option<&str> {
    self
      .inner
      .as_object_raw()
      .and_then(|o| o.as_str())
      .map(|o| o.as_str())
  }

  pub fn is_dict(&self) -> bool {
    self.inner.is_dict()
  }

  pub fn as_dict(self) -> Option<Dict<'a>> {
    self.inner.to_dict().map(Dict::bind)
  }

  pub(crate) fn is_object(&self) -> bool {
    self.inner.is_object()
  }
}

impl<'a> Value<'a> {
  pub(crate) fn bind(value: impl Into<CoreValue>) -> Value<'a> {
    Self {
      inner: value.into(),
      _lifetime: PhantomData,
    }
  }

  pub(crate) fn unbind(self) -> CoreValue {
    self.inner
  }
}

impl<'a> Display for Value<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.inner, f)
  }
}

#[repr(C)]
pub struct Str<'a> {
  inner: crate::value::handle::Handle<crate::value::object::Str>,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> Str<'a> {
  pub fn as_str_ref(&self) -> &str {
    self.inner.as_str()
  }
}

impl<'a> Str<'a> {
  pub(crate) fn bind(value: crate::value::handle::Handle<crate::value::object::Str>) -> Self {
    Self {
      inner: value,
      _lifetime: PhantomData,
    }
  }

  pub(crate) fn unbind(self) -> crate::value::handle::Handle<crate::value::object::Str> {
    self.inner
  }
}

pub trait IntoStr {
  fn into_str<'a>(self, ctx: &Context<'a>) -> Str<'a>;
}

impl<T> IntoStr for T
where
  object::Str: From<T>,
{
  fn into_str<'a>(self, ctx: &Context<'a>) -> Str<'a> {
    Str::bind(ctx.inner().alloc(object::Str::from(self)))
  }
}

#[repr(C)]
pub struct Dict<'a> {
  inner: crate::value::handle::Handle<crate::value::object::Dict>,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> Dict<'a> {
  pub(crate) fn bind(value: crate::value::handle::Handle<crate::value::object::Dict>) -> Self {
    Self {
      inner: value,
      _lifetime: PhantomData,
    }
  }

  pub fn iter(&'a self) -> impl Iterator<Item = (&'a str, &'a Value<'a>)> + 'a {
    self.inner.iter().map(|(k, v)| {
      (k.as_str(), unsafe {
        std::mem::transmute::<&CoreValue, &Value>(v)
      })
    })
  }

  pub fn has(&self, key: &str) -> bool {
    self.inner.contains_key(key)
  }

  pub fn get(&self, key: &str) -> Option<&Value<'a>> {
    self
      .inner
      .get(key)
      .map(|v| unsafe { std::mem::transmute::<&CoreValue, &Value>(v) })
  }

  pub fn set(&mut self, key: Str<'a>, value: Value<'a>) {
    self.inner.insert(key.inner, value.inner);
  }

  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  pub fn len(&self) -> usize {
    self.inner.len()
  }
}

impl<'a> Display for Dict<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.inner, f)
  }
}

impl<'a> Debug for Dict<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut s = f.debug_struct("Dict");
    s.finish()
  }
}
