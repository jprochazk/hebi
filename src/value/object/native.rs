use std::fmt::Display;
use std::mem::transmute;

use super::Access;
use crate::ctx::Context as CoreContext;
use crate::value::handle::Handle;
use crate::value::object::Dict as CoreDict;
use crate::value::Value as CoreValue;
use crate::{public, Result};

pub trait Callable {
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>;
}

impl<F> Callable for F
where
  F: for<'a> Fn(
    &'a public::Context<'a>,
    &'a [public::Value<'a>],
    Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>,
  F: Send + 'static,
{
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>> {
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
    kwargs: Option<Handle<CoreDict>>,
  ) -> Result<CoreValue> {
    let ctx = public::Context::bind(ctx);
    // Safety: `public::Value` is `repr(C)`, and holds a `CoreValue` + one
    // `PhantomData` field, so its layout is equivalent to `CoreValue`.
    let argv = unsafe { transmute::<&[CoreValue], &[public::Value]>(argv) };
    let kwargs = kwargs.map(public::Dict::bind);
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
