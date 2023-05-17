use std::future;
use std::future::Future;
use std::rc::Rc;
use std::string::String as StdString;
use std::sync::Arc;

use futures_util::TryFutureExt;
use indexmap::IndexMap;

use crate::object::native::{AsyncCallback, Callback};
use crate::{AsyncScope, Result, Unbind, Value};

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
  pub(crate) fns: IndexMap<StdString, Callback>,
  pub(crate) async_fns: IndexMap<StdString, AsyncCallback>,
}

pub struct NativeModuleBuilder {
  data: NativeModuleData,
}

impl NativeModuleBuilder {
  pub fn function(mut self, name: impl ToString, f: Callback) -> Self {
    self.data.fns.insert(name.to_string(), f);
    self
  }

  pub fn async_function<'cx, Fut>(
    mut self,
    name: impl ToString,
    f: fn(AsyncScope<'cx>) -> Fut,
  ) -> Self
  where
    Fut: Future<Output = Result<Value<'cx>>> + 'static,
  {
    self.data.async_fns.insert(
      name.to_string(),
      Rc::new(move |scope, _| {
        Box::pin(
          f(unsafe { std::mem::transmute::<_, AsyncScope<'static>>(scope) })
            .and_then(|value| future::ready(Ok(value.unbind()))),
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
