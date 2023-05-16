use std::string::String as StdString;
use std::sync::Arc;

use indexmap::IndexMap;

use crate::object::native::Callback;

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
      },
    }
  }
}

pub(crate) struct NativeModuleData {
  pub(crate) name: StdString,
  pub(crate) fns: IndexMap<StdString, Callback>,
}

pub struct NativeModuleBuilder {
  data: NativeModuleData,
}

impl NativeModuleBuilder {
  pub fn function(mut self, name: impl ToString, f: Callback) -> Self {
    self.data.fns.insert(name.to_string(), f);
    self
  }

  pub fn finish(self) -> NativeModule {
    NativeModule {
      data: Arc::new(self.data),
    }
  }
}
