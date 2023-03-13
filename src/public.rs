pub(crate) mod conv;

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

pub use self::core::object::native::{FunctionPtr, TypeInfo};
use self::core::{object, Value as CoreValue};
use crate::ctx::Context as CoreContext;
use crate::isolate::call::Args as CoreArgs;
use crate::{value as core, Error, Result};

// TODO: make macro for wrapping types
// - `T` -> `T<'a> { inner: T, _lifetime: PhantomData<&'a ()> }`
// - T<'a>::bind(T) -> T<'a>
// - Value::as_T(self) -> Option<T<'a>>
// - From<T<'a>> for Value<'a>

// TODO: `Context` type for allocating objects

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

  pub(crate) fn bind_ref(v: &CoreContext) -> &Context<'a> {
    // SAFETY: `CoreContext` and `Context` are layout-compatible
    // due to `Value` being `repr(C)`, and holding `CoreContext`
    // as its only non-ZST field
    unsafe { std::mem::transmute::<&CoreContext, &Context<'a>>(v) }
  }

  pub(crate) fn unbind(self) -> CoreContext {
    self.inner
  }
}

// FIXME: rust-analyzer doesn't understand `derive(Clone)`
impl<'a> Clone for Context<'a> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      _lifetime: self._lifetime,
    }
  }
}

#[repr(C)]
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
      .and_then(|o| o.as_str_ref())
      .map(|o| o.as_str())
  }

  pub fn is_dict(&self) -> bool {
    self.inner.is_dict()
  }

  pub fn as_dict(self) -> Option<Dict<'a>> {
    self.inner.to_dict().map(Dict::bind)
  }

  pub fn as_user_data(self) -> Option<UserData<'a>> {
    self.inner.to_user_data().map(UserData::bind)
  }

  pub(crate) fn is_object(&self) -> bool {
    self.inner.is_object()
  }
}

impl<'a> Value<'a> {
  pub fn none() -> Self {
    Value::bind(CoreValue::none())
  }
}

impl<'a> Value<'a> {
  pub(crate) fn bind_slice(values: &[CoreValue]) -> &[Value<'a>] {
    // SAFETY: `CoreValue` and `Value` are layout-compatible
    // due to `Value` being `repr(C)`, and holding `CoreValue`
    // as its only non-ZST field
    unsafe { std::mem::transmute::<&[CoreValue], &[Value<'a>]>(values) }
  }

  pub(crate) fn bind_ref(v: &CoreValue) -> &Value<'a> {
    // SAFETY: `CoreValue` and `Value` are layout-compatible
    // due to `Value` being `repr(C)`, and holding `CoreValue`
    // as its only non-ZST field
    unsafe { std::mem::transmute::<&CoreValue, &Value<'a>>(v) }
  }

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

// FIXME: rust-analyzer doesn't understand `derive(Clone)`
impl<'a> Clone for Value<'a> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      _lifetime: self._lifetime,
    }
  }
}

#[repr(C)]
pub struct Str<'a> {
  inner: core::handle::Handle<core::object::Str>,
  _lifetime: PhantomData<&'a ()>,
}

// FIXME: rust-analyzer doesn't understand `derive(Clone)`
impl<'a> Clone for Str<'a> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      _lifetime: self._lifetime,
    }
  }
}

impl<'a> Str<'a> {
  pub fn as_str_ref(&self) -> &str {
    self.inner.as_str()
  }
}

impl<'a> Str<'a> {
  pub(crate) fn bind(value: core::handle::Handle<core::object::Str>) -> Self {
    Self {
      inner: value,
      _lifetime: PhantomData,
    }
  }

  pub(crate) fn unbind(self) -> core::handle::Handle<core::object::Str> {
    self.inner
  }
}

pub trait IntoStr<'a> {
  fn into_str(self, ctx: &Context<'a>) -> Str<'a>;
}

impl<'a, T> IntoStr<'a> for T
where
  object::Str: From<T>,
{
  fn into_str(self, ctx: &Context<'a>) -> Str<'a> {
    Str::bind(ctx.inner().alloc(object::Str::from(self)))
  }
}

impl<'a> IntoStr<'a> for core::handle::Handle<core::object::Str> {
  fn into_str(self, _: &Context<'a>) -> Str<'a> {
    Str::bind(self)
  }
}

impl<'a> IntoStr<'a> for Str<'a> {
  fn into_str(self, _: &Context<'a>) -> Str<'a> {
    self
  }
}

#[repr(C)]
pub struct Dict<'a> {
  inner: core::handle::Handle<core::object::Dict>,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> Dict<'a> {
  pub(crate) fn bind(value: core::handle::Handle<core::object::Dict>) -> Self {
    Self {
      inner: value,
      _lifetime: PhantomData,
    }
  }

  pub fn iter(&'a self) -> impl Iterator<Item = (&'a str, &'a Value<'a>)> + 'a {
    self
      .inner
      .iter()
      .map(|(k, v)| (k.as_str(), Value::bind_ref(v)))
  }

  pub fn has(&self, key: &str) -> bool {
    self.inner.contains_key(key)
  }

  pub fn get(&self, key: &str) -> Option<&Value<'a>> {
    self.inner.get(key).map(Value::bind_ref)
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

// FIXME: rust-analyzer doesn't understand `derive(Clone)`
impl<'a> Clone for Dict<'a> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      _lifetime: self._lifetime,
    }
  }
}

#[repr(C)]
pub struct UserData<'a> {
  inner: core::handle::Handle<core::object::UserData>,
  _lifetime: PhantomData<&'a ()>,
}

impl<'a> UserData<'a> {
  pub fn new<T: TypeInfo + 'static>(ctx: &Context, v: T) -> Self {
    let inner = core::object::UserData::new(ctx.inner(), v);
    UserData {
      inner,
      _lifetime: PhantomData,
    }
  }

  #[doc(hidden)]
  pub unsafe fn cast<T: TypeInfo + 'static>(&self) -> Option<&T> {
    unsafe { self.inner._get().inner() }.as_any().downcast_ref()
  }

  #[doc(hidden)]
  pub unsafe fn cast_mut<T: TypeInfo + 'static>(&mut self) -> Option<&mut T> {
    unsafe { self.inner._get_mut().inner_mut() }
      .as_any_mut()
      .downcast_mut()
  }

  pub(crate) fn bind(inner: core::handle::Handle<core::object::UserData>) -> Self {
    Self {
      inner,
      _lifetime: PhantomData,
    }
  }

  pub(crate) fn unbind(self) -> core::handle::Handle<core::object::UserData> {
    self.inner
  }
}

impl<'a> Display for UserData<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.inner, f)
  }
}

impl<'a> Debug for UserData<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("UserData").finish()
  }
}

impl<'a> Clone for UserData<'a> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      _lifetime: self._lifetime,
    }
  }
}

/// Internal trait used by derive macros. Do not use directly.
#[doc(hidden)]
pub trait IntoUserData<'a> {
  fn into_user_data(self, ctx: &Context) -> Result<Value<'a>>;
}

impl<'a, T> IntoUserData<'a> for T
where
  T: TypeInfo + 'static,
{
  fn into_user_data(self, ctx: &Context) -> Result<Value<'a>> {
    Ok(Value::bind(UserData::new(ctx, self).unbind()))
  }
}

impl<'a, T> IntoUserData<'a> for Option<T>
where
  T: IntoUserData<'a>,
{
  fn into_user_data(self, ctx: &Context) -> Result<Value<'a>> {
    match self {
      Some(v) => v.into_user_data(ctx),
      None => Err(Error::runtime("failed to initialize user data")),
    }
  }
}

impl<'a, T, E> IntoUserData<'a> for Result<T, E>
where
  T: IntoUserData<'a>,
  E: Into<Error>,
{
  fn into_user_data(self, ctx: &Context) -> Result<Value<'a>> {
    match self {
      Ok(v) => v.into_user_data(ctx),
      Err(e) => Err(e.into()),
    }
  }
}

#[repr(C)]
pub struct Args<'a> {
  this: CoreValue,
  positional: &'a [CoreValue],
  keyword: Option<core::handle::Handle<core::object::Dict>>,
}

impl<'a> Args<'a> {
  pub(crate) fn new(
    this: CoreValue,
    positional: &'a [CoreValue],
    keyword: Option<core::handle::Handle<core::object::Dict>>,
  ) -> Self {
    Self {
      this,
      positional,
      keyword,
    }
  }

  pub fn this(&self) -> Value<'a> {
    Value::bind(self.this.clone())
  }

  pub fn positional(&self) -> &[Value<'a>] {
    Value::bind_slice(self.positional)
  }

  pub fn keyword(&self) -> Option<Dict<'a>> {
    self.keyword.clone().map(Dict::bind)
  }
}
