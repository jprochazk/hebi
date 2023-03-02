use std::fmt::Display;
use std::mem::transmute;

use super::{Access, Str};
use crate::ctx::Context as CoreContext;
use crate::value::handle::Handle;
use crate::value::object::Dict as CoreDict;
use crate::value::Value as CoreValue;
use crate::{public, RuntimeError};

pub trait Callable {
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: &'a public::Dict<'a>,
  ) -> Result<public::Value<'a>, RuntimeError>;
}

impl<F> Callable for F
where
  F: for<'a> Fn(
    &'a public::Context<'a>,
    &'a [public::Value<'a>],
    &'a public::Dict<'a>,
  ) -> Result<public::Value<'a>, RuntimeError>,
  F: Send + 'static,
{
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: &'a public::Dict<'a>,
  ) -> Result<public::Value<'a>, RuntimeError> {
    self(ctx, argv, kwargs)
  }
}

pub struct NativeFunction {
  f: Box<dyn Callable>,
}

impl NativeFunction {
  pub fn new(f: Box<dyn Callable>) -> Self {
    Self { f }
  }
}

#[derive::delegate_to_handle]
impl NativeFunction {
  pub fn call(
    &self,
    ctx: CoreContext,
    argv: &[CoreValue],
    kwargs: &CoreDict,
  ) -> Result<CoreValue, RuntimeError> {
    let ctx = public::Context::bind(ctx);
    // Safety: `public::Value` is `repr(C)`, and holds a `CoreValue` + one
    // `PhantomData` field, so its layout is equivalent to `CoreValue`.
    let argv = unsafe { transmute::<&[CoreValue], &[public::Value]>(argv) };
    // Safety: same as above, but for `public::Dict`/`CoreDict`.
    let kwargs = unsafe { transmute::<&CoreDict, &public::Dict>(kwargs) };
    let result = self.f.call(&ctx, argv, kwargs)?;
    Ok(result.unbind())
  }
}

impl Access for NativeFunction {}

impl Display for NativeFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native function>")
  }
}
