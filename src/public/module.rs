use std::any::TypeId;
use std::future;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::transmute;
use std::string::String as StdString;
use std::sync::Arc;

use futures_util::{FutureExt, TryFutureExt};
use indexmap::IndexMap;

use crate::internal::error::Result;
use crate::internal::object::native::{
  AsyncCallback, NativeClassDescriptor, NativeClassInstance, NativeFieldDescriptor,
  NativeMethodDescriptor, SyncCallback,
};
use crate::internal::value::Value as OwnedValue;
use crate::internal::vm::thread::Args;
use crate::public::{FromValue, IntoValue, Scope, This, Unbind, Value};

#[derive(Clone)]
pub struct NativeModule {
  pub(crate) data: Arc<NativeModuleData>,
}

impl NativeModule {
  pub fn builder(name: impl ToString) -> NativeModuleBuilder {
    NativeModuleBuilder {
      data: NativeModuleData {
        name: name.to_string(),
        fns: IndexMap::new(),
        async_fns: IndexMap::new(),
        classes: IndexMap::new(),
      },
    }
  }
}

pub(crate) struct NativeModuleData {
  pub(crate) name: StdString,
  pub(crate) fns: IndexMap<StdString, SyncCallback>,
  pub(crate) async_fns: IndexMap<StdString, AsyncCallback>,
  pub(crate) classes: IndexMap<StdString, NativeClassDescriptor>,
}

pub struct NativeModuleBuilder {
  data: NativeModuleData,
}

impl NativeModuleBuilder {
  pub fn function<'cx, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>) -> R + Send + Sync + 'static,
  ) -> Self
  where
    R: IntoValue<'cx> + 'static,
  {
    self.data.fns.insert(name.to_string(), wrap_fn(f));
    self
  }

  pub fn async_function<'cx, Fut, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>) -> Fut + Send + Sync + 'static,
  ) -> Self
  where
    Fut: Future<Output = R> + 'static,
    R: IntoValue<'cx>,
  {
    self
      .data
      .async_fns
      .insert(name.to_string(), wrap_async_fn(f));
    self
  }

  pub fn class<T: Send + 'static>(
    mut self,
    name: impl ToString,
    f: impl Fn(NativeClassBuilder<false, T>) -> NativeClassDescriptor + Send + Sync + 'static,
  ) -> Self {
    let name = name.to_string();
    self
      .data
      .classes
      .insert(name.clone(), f(NativeClassBuilder::new(name)));
    self
  }

  pub fn finish(self) -> NativeModule {
    NativeModule {
      data: Arc::new(self.data),
    }
  }
}

pub struct NativeClassBuilder<const HAS_INIT: bool, T: Send> {
  descriptor: NativeClassDescriptor,
  ty: PhantomData<fn() -> T>,
}

impl<T: Send + 'static> NativeClassBuilder<false, T> {
  pub fn init(
    mut self,
    f: impl Fn(Scope<'_>) -> Result<T> + Send + Sync + 'static,
  ) -> NativeClassBuilder<true, T> {
    self.descriptor.init = Some(wrap_fn(move |scope| {
      let global = scope.global();
      global.new_instance(f(scope)?)
    }));
    NativeClassBuilder {
      descriptor: self.descriptor,
      ty: self.ty,
    }
  }
}

impl<const HAS_INIT: bool, T: Send + 'static> NativeClassBuilder<HAS_INIT, T> {
  pub fn new(name: StdString) -> Self {
    Self {
      descriptor: NativeClassDescriptor {
        name,
        type_id: TypeId::of::<T>(),
        init: None,
        fields: IndexMap::new(),
        methods: IndexMap::new(),
        static_methods: IndexMap::new(),
      },
      ty: PhantomData,
    }
  }

  pub fn finish(self) -> NativeClassDescriptor {
    self.descriptor
  }

  pub fn field<'cx, G, V>(mut self, name: impl ToString, get: G) -> Self
  where
    G: Fn(Scope<'cx>, This<'cx, T>) -> V + Send + Sync + 'static,
    V: IntoValue<'cx> + 'static,
  {
    self.descriptor.fields.insert(
      name.to_string(),
      NativeFieldDescriptor {
        get: wrap_getter(get),
        set: None,
      },
    );
    self
  }

  pub fn field_mut<'cx, G, S, V>(mut self, name: impl ToString, get: G, set: S) -> Self
  where
    G: Fn(Scope<'cx>, This<'cx, T>) -> V + Send + Sync + 'static,
    S: Fn(Scope<'cx>, This<'cx, T>, V) -> Result<()> + Send + Sync + 'static,
    V: IntoValue<'cx> + FromValue<'cx> + 'static,
  {
    self.descriptor.fields.insert(
      name.to_string(),
      NativeFieldDescriptor {
        get: wrap_getter(get),
        set: Some(wrap_setter(set)),
      },
    );
    self
  }

  pub fn method<'cx, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>, This<'cx, T>) -> R + Send + Sync + 'static,
  ) -> Self
  where
    R: IntoValue<'cx>,
  {
    self.descriptor.methods.insert(
      name.to_string(),
      NativeMethodDescriptor::Sync(wrap_method(f)),
    );
    self
  }

  pub fn async_method<'cx, Fut, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>, This<'cx, T>) -> Fut + Send + Sync + 'static,
  ) -> Self
  where
    Fut: Future<Output = R> + 'static,
    R: IntoValue<'cx>,
  {
    self.descriptor.methods.insert(
      name.to_string(),
      NativeMethodDescriptor::Async(wrap_async_method(f)),
    );
    self
  }

  pub fn static_method<'cx, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>) -> R + Send + Sync + 'static,
  ) -> Self
  where
    R: IntoValue<'cx> + 'static,
  {
    self
      .descriptor
      .static_methods
      .insert(name.to_string(), NativeMethodDescriptor::Sync(wrap_fn(f)));
    self
  }
}

fn wrap_fn<'cx, R>(f: impl Fn(Scope<'cx>) -> R + Send + Sync + 'static) -> SyncCallback
where
  R: IntoValue<'cx> + 'static,
{
  Arc::new(move |scope| {
    let scope = unsafe { transmute::<_, Scope<'static>>(scope) };
    let global = scope.global();
    f(scope).into_value(global).map(|value| value.unbind())
  })
}

fn wrap_async_fn<'cx, Fut, R>(
  f: impl Fn(Scope<'cx>) -> Fut + Send + Sync + 'static,
) -> AsyncCallback
where
  Fut: Future<Output = R> + 'static,
  R: IntoValue<'cx>,
{
  Arc::new(move |scope| {
    let scope = unsafe { transmute::<_, Scope<'static>>(scope) };
    let global = scope.global();
    Box::pin(
      f(scope)
        .map(|value| value.into_value(global))
        .map_ok(|value| value.unbind()),
    )
  })
}

fn extract_this<T: Send + 'static>(scope: Scope<'_>) -> Result<(Scope<'_>, This<'_, T>)> {
  let this = scope
    .param::<Value>(0)?
    .unbind()
    .to_object::<NativeClassInstance>()
    .ok_or_else(|| {
      error!(
        "receiver is not an instance of {}",
        std::any::type_name::<T>()
      )
    })?;
  let this = This::new(this).ok_or_else(|| {
    error!(
      "receiver is not an instance of {}",
      std::any::type_name::<T>()
    )
  })?;
  let scope = Scope {
    thread: scope.thread,
    args: Args {
      start: scope.args.start + 1,
      count: scope.args.count - 1,
    },
    stack_base: scope.stack_base,
    lifetime: PhantomData,
  };
  Ok((scope, this))
}

fn wrap_method<'cx, T: Send + 'static, R>(
  f: impl Fn(Scope<'cx>, This<'cx, T>) -> R + Send + Sync + 'static,
) -> SyncCallback
where
  R: IntoValue<'cx>,
{
  Arc::new(move |scope| {
    let (scope, this) = extract_this::<T>(scope)?;
    let (scope, this) =
      unsafe { transmute::<_, (Scope<'static>, This<'static, T>)>((scope, this)) };
    let global = scope.global();
    f(scope, this)
      .into_value(global)
      .map(|value| value.unbind())
  })
}

fn wrap_async_method<'cx, T: Send + 'static, Fut, R>(
  f: impl Fn(Scope<'cx>, This<'cx, T>) -> Fut + Send + Sync + 'static,
) -> AsyncCallback
where
  Fut: Future<Output = R> + 'static,
  R: IntoValue<'cx>,
{
  Arc::new(move |scope| {
    let (scope, this) = match extract_this::<T>(scope) {
      Ok(v) => v,
      Err(e) => return Box::pin(future::ready(Err(e))),
    };
    let (scope, this) =
      unsafe { transmute::<_, (Scope<'static>, This<'static, T>)>((scope, this)) };
    let global = scope.global();
    Box::pin(
      f(scope, this)
        .map(|value| value.into_value(global))
        .map_ok(|value| value.unbind()),
    )
  })
}

fn wrap_getter<'cx, T: Send + 'static, R>(
  f: impl Fn(Scope<'cx>, This<'cx, T>) -> R + Send + Sync + 'static,
) -> SyncCallback
where
  R: IntoValue<'cx> + 'static,
{
  Arc::new(move |scope| {
    let (scope, this) = extract_this::<T>(scope)?;
    let (scope, this) =
      unsafe { transmute::<_, (Scope<'static>, This<'static, T>)>((scope, this)) };
    if scope.args.count > 0 {
      fail!("getter called with argument");
    }
    let global = scope.global();
    f(scope, this)
      .into_value(global)
      .map(|value| value.unbind())
  })
}

fn wrap_setter<'cx, T: Send + 'static, V>(
  f: impl Fn(Scope<'cx>, This<'cx, T>, V) -> Result<()> + Send + Sync + 'static,
) -> SyncCallback
where
  V: FromValue<'cx> + 'static,
{
  Arc::new(move |scope| {
    let (scope, this) = extract_this::<T>(scope)?;
    let (scope, this) =
      unsafe { transmute::<_, (Scope<'static>, This<'static, T>)>((scope, this)) };

    let value = scope.param::<V>(0)?;
    let mut scope = scope;
    scope.args.start += 1;
    scope.args.count -= 1;

    f(scope, this, value)?;
    Ok(OwnedValue::none())
  })
}
