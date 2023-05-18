use std::future::Future;
use std::rc::Rc;
use std::string::String as StdString;
use std::sync::Arc;

use futures_util::{FutureExt, TryFutureExt};
use indexmap::IndexMap;

use crate::object::native::{AsyncCallback, SyncCallback};
use crate::{IntoValue, Scope, Unbind};

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
      },
    }
  }
}

pub(crate) struct NativeModuleData {
  pub(crate) name: StdString,
  pub(crate) fns: IndexMap<StdString, SyncCallback>,
  pub(crate) async_fns: IndexMap<StdString, AsyncCallback>,
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
    self.data.fns.insert(
      name.to_string(),
      Rc::new(move |scope| {
        let scope = unsafe { std::mem::transmute::<_, Scope<'static>>(scope) };
        let cx = scope.cx();
        f(scope).into_value(cx).map(|value| value.unbind())
      }),
    );
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
    self.data.async_fns.insert(
      name.to_string(),
      Rc::new(move |scope| {
        let scope = unsafe { std::mem::transmute::<_, Scope<'static>>(scope) };
        let cx = scope.cx();
        Box::pin(
          f(scope)
            .map(|value| value.into_value(cx))
            .map_ok(|value| value.unbind()),
        )
      }),
    );
    self
  }

  pub fn finish(self) -> NativeModule {
    NativeModule {
      data: Arc::new(self.data),
    }
  }
}
