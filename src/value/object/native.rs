use std::fmt::{Debug, Display};
use std::mem::transmute;

use indexmap::IndexMap;

use super::{Access, Str};
use crate::ctx::Context as CoreContext;
use crate::value::handle::Handle;
use crate::value::object::Dict as CoreDict;
use crate::value::Value as CoreValue;
use crate::{public, Error, Result};

pub trait Function: dyn_clone::DynClone {
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>;
}

dyn_clone::clone_trait_object!(Function);

impl<F> Function for F
where
  F: for<'a> Fn(
    &'a public::Context<'a>,
    &'a [public::Value<'a>],
    Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>,
  F: Send + 'static,
  F: Clone,
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
  f: Box<dyn Function>,
}

impl NativeFunction {
  pub fn new(f: Box<dyn Function>) -> Self {
    Self { f }
  }
}

#[derive::delegate_to_handle]
impl NativeFunction {
  pub fn call(
    &self,
    ctx: &CoreContext,
    argv: &[CoreValue],
    kwargs: Option<Handle<CoreDict>>,
  ) -> Result<CoreValue> {
    // Safety: `public::Context` is `repr(C)`, and holds a `CoreContext` + one
    // `PhantomData` field, so its layout is equivalent to `CoreContext`.
    let ctx = unsafe { transmute::<&CoreContext, &public::Context>(ctx) };
    // Safety: `public::Value` is `repr(C)`, and holds a `CoreValue` + one
    // `PhantomData` field, so its layout is equivalent to `CoreValue`.
    let argv = unsafe { transmute::<&[CoreValue], &[public::Value]>(argv) };
    let kwargs = kwargs.map(public::Dict::bind);
    let result = self.f.call(ctx, argv, kwargs)?;
    Ok(result.unbind())
  }
}

impl Access for NativeFunction {}

impl Display for NativeFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native function>")
  }
}

pub struct NativeAccessorDescriptor {
  pub name: &'static str,
  pub get: &'static dyn Function,
  pub set: Option<&'static dyn Function>,
}

pub trait NativeAccessorDescriptors {
  fn accessor_descriptors() -> &'static [NativeAccessorDescriptor];
}

pub struct NativeMethodDescriptor {
  pub name: &'static str,
  pub f: &'static dyn Function,
}

pub trait NativeMethodDescriptors {
  fn init() -> Option<&'static dyn Function>;
  fn method_descriptors() -> &'static [NativeMethodDescriptor];
}

pub struct Accessor {
  get: Handle<NativeFunction>,
  set: Option<Handle<NativeFunction>>,
}

pub struct UserData {
  data: Box<dyn std::any::Any>,
}

#[derive::delegate_to_handle]
impl UserData {
  pub(crate) fn data(&self) -> &dyn std::any::Any {
    &self.data
  }

  pub(crate) fn data_mut(&mut self) -> &mut dyn std::any::Any {
    &mut self.data
  }
}

impl Access for UserData {}

impl Debug for UserData {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("UserData").finish()
  }
}

impl Display for UserData {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<user data>")
  }
}

pub struct NativeClass {
  name: Handle<Str>,
  init: Option<Handle<NativeFunction>>,
  accessors: IndexMap<&'static str, Accessor>,
  methods: IndexMap<&'static str, Handle<NativeFunction>>,
}

#[derive::delegate_to_handle]
impl NativeClass {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn init(&self) -> Option<Handle<NativeFunction>> {
    self.init.clone()
  }

  pub(crate) fn accessors(&self) -> &IndexMap<&'static str, Accessor> {
    &self.accessors
  }

  pub(crate) fn methods(&self) -> &IndexMap<&'static str, Handle<NativeFunction>> {
    &self.methods
  }
}

impl Access for NativeClass {}

impl Debug for NativeClass {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NativeClass").finish()
  }
}

impl Display for NativeClass {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native class {}>", self.name)
  }
}

pub struct NativeClassInstance {
  class: Handle<NativeClass>,
  data: Handle<UserData>,
}

impl NativeClassInstance {
  pub fn new(
    ctx: &CoreContext,
    class: Handle<NativeClass>,
    data: Handle<UserData>,
  ) -> Handle<Self> {
    ctx.alloc(Self { class, data })
  }
}

impl Access for NativeClassInstance {
  fn is_frozen(&self) -> bool {
    false
  }

  fn should_bind_methods(&self) -> bool {
    true
  }

  fn field_get(&self, ctx: &CoreContext, key: &str) -> Result<Option<CoreValue>> {
    if let Some(get) = self.class.accessors().get(key).map(|a| &a.get) {
      let result = get.call(ctx, &[self.data.clone().into()], None)?;
      return Ok(Some(result));
    }

    if let Some(method) = self.class.methods().get(key).cloned() {
      return Ok(Some(method.into()));
    }

    Ok(None)
  }

  fn field_set(
    &mut self,
    ctx: &CoreContext,
    key: Handle<super::Str>,
    value: CoreValue,
  ) -> Result<()> {
    if let Some(set) = self
      .class
      .accessors()
      .get(key.as_str())
      .and_then(|a| a.set.as_ref())
    {
      set.call(ctx, &[self.data.clone().into(), value], None)?;
    }

    Err(Error::runtime(format!("cannot set field `{key}`")))
  }
}

impl Debug for NativeClassInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("NativeClassInstance").finish()
  }
}

impl Display for NativeClassInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native class instance>")
  }
}

// #[class]
struct Test {
  // #[access(get)]
  value: i32,
}

// #[methods]
impl Test {
  // #[new]
  fn new(value: i32) -> Self {
    Test { value }
  }

  fn square(&self) -> i32 {
    self.value * self.value
  }
}

impl NativeAccessorDescriptors for Test {
  fn accessor_descriptors() -> &'static [NativeAccessorDescriptor] {
    use crate::{FromHebi, IntoHebi};

    fn _Test__get__value<'a>(
      ctx: &'a public::Context<'a>,
      argv: &'a [public::Value<'a>],
      kwargs: Option<public::Dict<'a>>,
    ) -> Result<public::Value<'a>> {
      // gracefully handle this
      let this = argv[0].clone().unbind().to_user_data().unwrap();
      let this = this.data().downcast_ref::<Test>().unwrap();
      let value = this.value;
      value.into_hebi(ctx)
    }

    &[NativeAccessorDescriptor {
      name: "value",
      get: &_Test__get__value,
      set: None,
    }]
  }
}

impl NativeMethodDescriptors for Test {
  fn init() -> Option<&'static dyn Function> {
    use crate::{FromHebi, IntoHebi};

    fn _Test__call__new<'a>(
      ctx: &'a public::Context<'a>,
      argv: &'a [public::Value<'a>],
      kwargs: Option<public::Dict<'a>>,
    ) -> Result<public::Value<'a>> {
      let _0 = i32::from_hebi(ctx, argv[0].clone())?;
      // should also support fallible `new`
      let data = Box::new(Test::new(_0));
      Ok(public::Value::bind(ctx.inner().alloc(UserData { data })))
    }

    Some(&_Test__call__new)
  }

  fn method_descriptors() -> &'static [NativeMethodDescriptor] {
    use crate::{FromHebi, IntoHebi};

    fn _Test__call__square<'a>(
      ctx: &'a public::Context<'a>,
      argv: &'a [public::Value<'a>],
      kwargs: Option<public::Dict<'a>>,
    ) -> Result<public::Value<'a>> {
      // gracefully handle this
      let this = argv[0].clone().unbind().to_user_data().unwrap();
      let this = this.data().downcast_ref::<Test>().unwrap();
      let value = this.square();
      value.into_hebi(ctx)
    }

    &[NativeMethodDescriptor {
      name: "square",
      f: &_Test__call__square,
    }]
  }
}
