use std::fmt::{Debug, Display};
use std::mem::transmute;

use indexmap::IndexMap;

use super::{Access, Str};
use crate::ctx::Context as CoreContext;
use crate::value::handle::Handle;
use crate::value::object::Dict as CoreDict;
use crate::value::Value as CoreValue;
use crate::{public, Error, Result};

pub trait Function {
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>;
}

impl<F> Function for F
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

pub type FunctionPtr = for<'a> fn(
  &'a public::Context<'a>,
  &'a [public::Value<'a>],
  Option<public::Dict<'a>>,
) -> Result<public::Value<'a>>;

pub struct NativeFunction {
  name: Handle<Str>,
  f: Box<dyn Function>,
}

impl NativeFunction {
  pub fn new(ctx: &CoreContext, name: Handle<Str>, f: Box<dyn Function>) -> Handle<NativeFunction> {
    ctx.alloc(Self { name, f })
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
    let ctx = public::Context::bind_ref(ctx);
    let argv = public::Value::bind_slice(argv);
    let kwargs = kwargs.map(public::Dict::bind);
    let result = Function::call(&*self.f, ctx, argv, kwargs)?;
    Ok(result.unbind())
  }

  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }
}

impl Access for NativeFunction {}

impl Display for NativeFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native function>")
  }
}

pub trait Method {
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    this: public::UserData<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>;
}

impl<F> Method for F
where
  F: for<'a> Fn(
    &'a public::Context<'a>,
    public::UserData<'a>,
    &'a [public::Value<'a>],
    Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>>,
  F: Send + 'static,
{
  fn call<'a>(
    &self,
    ctx: &'a public::Context<'a>,
    this: public::UserData<'a>,
    argv: &'a [public::Value<'a>],
    kwargs: Option<public::Dict<'a>>,
  ) -> Result<public::Value<'a>> {
    self(ctx, this, argv, kwargs)
  }
}

pub type MethodFnPtr = for<'a> fn(
  &'a public::Context<'a>,
  public::UserData<'a>,
  &'a [public::Value<'a>],
  Option<public::Dict<'a>>,
) -> Result<public::Value<'a>>;

pub struct NativeClassMethod {
  name: Handle<Str>,
  f: MethodFnPtr,
}

impl NativeClassMethod {
  pub fn new(ctx: &CoreContext, name: Handle<Str>, f: MethodFnPtr) -> Handle<NativeClassMethod> {
    ctx.alloc(Self { name, f })
  }
}

#[derive::delegate_to_handle]
impl NativeClassMethod {
  pub fn call(
    &self,
    ctx: &CoreContext,
    this: Handle<UserData>,
    argv: &[CoreValue],
    kwargs: Option<Handle<CoreDict>>,
  ) -> Result<CoreValue> {
    let ctx = public::Context::bind_ref(ctx);
    let argv = public::Value::bind_slice(argv);
    let kwargs = kwargs.map(public::Dict::bind);
    let result = Method::call(&self.f, ctx, public::UserData::bind(this), argv, kwargs)?;
    Ok(result.unbind())
  }

  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }
}

impl Access for NativeClassMethod {}

impl Display for NativeClassMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<native method {}>", self.name)
  }
}

pub trait AsUserData: std::any::Any {
  fn as_any(&self) -> &dyn std::any::Any;
  fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait TypeInfo {
  fn name() -> &'static str;
  /// The init function must return a `UserData`.
  fn init() -> Option<FunctionPtr>;
  fn fields() -> &'static [(&'static str, MethodFnPtr, Option<MethodFnPtr>)];
  fn methods() -> &'static [(&'static str, MethodFnPtr)];
  fn static_methods() -> &'static [(&'static str, FunctionPtr)];
}

impl<T: TypeInfo + 'static> AsUserData for T {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

// TODO typeid

pub struct UserData {
  inner: Box<dyn AsUserData>,
}

impl UserData {
  pub fn new<T: AsUserData>(ctx: &CoreContext, v: T) -> Handle<Self> {
    ctx.alloc(Self { inner: Box::new(v) })
  }
}

#[derive::delegate_to_handle]
impl UserData {
  pub(crate) unsafe fn inner(&self) -> &dyn AsUserData {
    &*self.inner
  }

  pub(crate) unsafe fn inner_mut(&mut self) -> &mut dyn AsUserData {
    &mut *self.inner
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

struct Accessor {
  get: Handle<NativeClassMethod>,
  set: Option<Handle<NativeClassMethod>>,
}

pub struct NativeClass {
  name: Handle<Str>,
  init: Option<FunctionPtr>,
  accessors: IndexMap<&'static str, Accessor>,
  methods: IndexMap<&'static str, Handle<NativeClassMethod>>,
  static_methods: IndexMap<&'static str, Handle<NativeFunction>>,
}

impl NativeClass {
  pub fn new<T: TypeInfo>(ctx: &CoreContext) -> Handle<NativeClass> {
    ctx.alloc(NativeClass {
      name: ctx.alloc(Str::from(T::name())),
      init: T::init(),
      accessors: T::fields()
        .iter()
        .map(|field| {
          (
            field.0,
            Accessor {
              get: NativeClassMethod::new(
                ctx,
                ctx.alloc(Str::from(format!("get {}", field.0))),
                field.1,
              ),
              set: field.2.map(|f| {
                NativeClassMethod::new(ctx, ctx.alloc(Str::from(format!("set {}", field.0))), f)
              }),
            },
          )
        })
        .collect(),
      methods: T::methods()
        .iter()
        .map(|m| {
          (
            m.0,
            NativeClassMethod::new(ctx, ctx.alloc(Str::from(m.0)), m.1),
          )
        })
        .collect(),
      static_methods: T::static_methods()
        .iter()
        .map(|m| {
          (
            m.0,
            NativeFunction::new(ctx, ctx.alloc(Str::from(m.0)), Box::new(m.1)),
          )
        })
        .collect(),
    })
  }
}

#[derive::delegate_to_handle]
impl NativeClass {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub(crate) fn init(&self) -> Option<FunctionPtr> {
    self.init
  }

  pub fn field_getter(&self, key: &str) -> Option<Handle<NativeClassMethod>> {
    self.accessors.get(key).map(|a| a.get.clone())
  }

  pub fn field_setter(&self, key: &str) -> Option<Handle<NativeClassMethod>> {
    self.accessors.get(key).and_then(|a| a.set.clone())
  }

  pub fn method(&self, key: &str) -> Option<Handle<NativeClassMethod>> {
    self.methods.get(key).cloned()
  }

  pub fn static_method(&self, key: &str) -> Option<Handle<NativeFunction>> {
    self.static_methods.get(key).cloned()
  }
}

impl Access for NativeClass {
  fn should_bind_methods(&self) -> bool {
    false
  }

  fn field_get(&self, _: &CoreContext, key: &str) -> Result<Option<CoreValue>> {
    if let Some(method) = self.method(key) {
      Ok(Some(method.into()))
    } else if let Some(static_method) = self.static_method(key) {
      Ok(Some(static_method.into()))
    } else {
      Ok(None)
    }
  }
}

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
  user_data: Handle<UserData>,
}

impl NativeClassInstance {
  pub fn new(
    ctx: &CoreContext,
    class: Handle<NativeClass>,
    user_data: Handle<UserData>,
  ) -> Handle<Self> {
    ctx.alloc(Self { class, user_data })
  }
}

#[derive::delegate_to_handle]
impl NativeClassInstance {
  pub(crate) fn class(&self) -> Handle<NativeClass> {
    self.class.clone()
  }

  pub(crate) fn user_data(&self) -> Handle<UserData> {
    self.user_data.clone()
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
    if let Some(get) = self.class.field_getter(key) {
      let result = get.call(ctx, self.user_data.clone(), &[], None)?;
      return Ok(Some(result));
    }

    if let Some(method) = self.class.method(key) {
      return Ok(Some(method.into()));
    }

    Ok(None)
  }

  fn field_set(&mut self, ctx: &CoreContext, key: Handle<Str>, value: CoreValue) -> Result<()> {
    if let Some(set) = self.class.field_setter(key.as_str()) {
      set.call(ctx, self.user_data.clone(), &[value], None)?;
      return Ok(());
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
    write!(f, "<native class {} instance>", self.class.name())
  }
}
