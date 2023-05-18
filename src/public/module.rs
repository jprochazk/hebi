use std::any::{Any as StdAny, TypeId};
use std::future::Future;
use std::marker::PhantomData;
use std::mem::transmute;
use std::rc::Rc;
use std::string::String as StdString;

use futures_util::{FutureExt, TryFutureExt};
use indexmap::IndexMap;

use super::ForceSendFuture;
use crate::object::native::{
  AsyncCallback, NativeClassDescriptor, NativeClassInstance, NativeFieldDescriptor,
  NativeMethodDescriptor, SyncCallback,
};
use crate::value::Value as OwnedValue;
use crate::vm::thread::Args;
use crate::{FromValue, IntoValue, Result, Scope, This, Unbind, Value};

#[derive(Clone)]
pub struct NativeModule {
  pub(crate) data: Rc<NativeModuleData>,
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
    f: impl Fn(Scope<'cx>) -> R + 'static,
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
    f: impl Fn(Scope<'cx>) -> Fut + 'static,
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

  pub fn class<T>(
    mut self,
    name: impl ToString,
    f: impl FnOnce(NativeClassBuilder<T>) -> NativeClassBuilder<T>,
  ) -> Self
  where
    T: StdAny,
  {
    let name = name.to_string();
    self
      .data
      .classes
      .insert(name.clone(), f(NativeClassBuilder::new(name)).finish());
    self
  }

  pub fn finish(self) -> NativeModule {
    NativeModule {
      data: Rc::new(self.data),
    }
  }
}

pub struct NativeClassBuilder<T: StdAny> {
  descriptor: NativeClassDescriptor,
  ty: PhantomData<fn() -> T>,
}

impl<T: StdAny> NativeClassBuilder<T> {
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

  fn finish(self) -> NativeClassDescriptor {
    self.descriptor
  }

  pub fn field<'cx, G, V>(mut self, name: impl ToString, get: G) -> Self
  where
    G: Fn(Scope<'cx>, This<'cx, T>) -> V + 'static,
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
    G: Fn(Scope<'cx>, This<'cx, T>) -> V + 'static,
    S: Fn(Scope<'cx>, This<'cx, T>, V) -> Result<()> + 'static,
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

  pub fn init(mut self, f: impl Fn(Scope<'_>) -> Result<T> + 'static) -> Self {
    // TODO: type safety
    if self.descriptor.init.is_some() {
      panic!("double init")
    }
    self.descriptor.init = Some(wrap_fn(move |scope| {
      let cx = scope.cx();
      cx.new_instance(f(scope)?)
    }));
    self
  }

  pub fn method<'cx, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>, This<'cx, T>) -> R + 'static,
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

  pub fn static_method<'cx, R>(
    mut self,
    name: impl ToString,
    f: impl Fn(Scope<'cx>) -> R + 'static,
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

fn wrap_fn<'cx, R>(f: impl Fn(Scope<'cx>) -> R + 'static) -> SyncCallback
where
  R: IntoValue<'cx> + 'static,
{
  Rc::new(move |scope| {
    let scope = unsafe { transmute::<_, Scope<'static>>(scope) };
    let cx = scope.cx();
    f(scope).into_value(cx).map(|value| value.unbind())
  })
}

fn wrap_async_fn<'cx, Fut, R>(f: impl Fn(Scope<'cx>) -> Fut + 'static) -> AsyncCallback
where
  Fut: Future<Output = R> + 'static,
  R: IntoValue<'cx>,
{
  Rc::new(move |scope| {
    let scope = unsafe { transmute::<_, Scope<'static>>(scope) };
    let cx = scope.cx();
    Box::pin(unsafe {
      ForceSendFuture::new(
        f(scope)
          .map(|value| value.into_value(cx))
          .map_ok(|value| value.unbind()),
      )
    })
  })
}

fn extract_this<T: 'static>(scope: Scope<'_>) -> Result<(Scope<'_>, This<'_, T>)> {
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
    lifetime: PhantomData,
  };
  Ok((scope, this))
}

fn wrap_method<'cx, T: 'static, R>(
  f: impl Fn(Scope<'cx>, This<'cx, T>) -> R + 'static,
) -> SyncCallback
where
  R: IntoValue<'cx>,
{
  Rc::new(move |scope| {
    let (scope, this) = extract_this::<T>(scope)?;
    let (scope, this) =
      unsafe { transmute::<_, (Scope<'static>, This<'static, T>)>((scope, this)) };
    let cx = scope.cx();
    f(scope, this).into_value(cx).map(|value| value.unbind())
  })
}

fn wrap_getter<'cx, T: 'static, R>(
  f: impl Fn(Scope<'cx>, This<'cx, T>) -> R + 'static,
) -> SyncCallback
where
  R: IntoValue<'cx> + 'static,
{
  Rc::new(move |scope| {
    let (scope, this) = extract_this::<T>(scope)?;
    let (scope, this) =
      unsafe { transmute::<_, (Scope<'static>, This<'static, T>)>((scope, this)) };
    if scope.args.count > 0 {
      fail!("getter called with argument");
    }
    let cx = scope.cx();
    f(scope, this).into_value(cx).map(|value| value.unbind())
  })
}

fn wrap_setter<'cx, T: 'static, V>(
  f: impl Fn(Scope<'cx>, This<'cx, T>, V) -> Result<()> + 'static,
) -> SyncCallback
where
  V: FromValue<'cx> + 'static,
{
  Rc::new(move |scope| {
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
