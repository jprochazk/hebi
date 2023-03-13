#![allow(clippy::wrong_self_convention)]

mod builtins;
mod ctx;
mod emit;
mod error;
mod isolate;
mod op;
pub mod public;
pub mod util;
mod value;

use std::any;
use std::cell::{Ref, RefCell};
use std::fmt::{Debug, Display};

pub use error::Error;
use indexmap::IndexMap;
use isolate::{Isolate, Stdout};
use public::{IntoStr, TypeInfo};
use value::Value as CoreValue;

pub type Result<T, E = Error> = std::result::Result<T, E>;

use ctx::Context;
pub use derive::{class, function, methods};
pub use public::conv::{FromHebi, FromHebiRef, IntoHebi};
pub use public::Value;
use value::handle::Handle;
pub use value::object::module::ModuleLoader;
use value::object::native::Function;
use value::object::{NativeClass, NativeClassInstance, NativeFunction, UserData};

pub struct Hebi {
  isolate: RefCell<Isolate>,
  classes: RefCell<IndexMap<any::TypeId, Handle<NativeClass>>>,
}

// # Safety:
// Internally, the VM uses reference counting using the `Rc` type.
// Normally, it would be unsound to implement Send for something that
// uses `Rc`, but in this case, the user can *never* obtain an internal
// `Rc` (or equivalent). This means they can never clone that `Rc`, and
// then move their `Hebi` instance to another thread, causing a data race
// between the user's clone of the `Rc` and Hebi's clone.
//
// This is enforced by via the `conv::Value<'a>` type, which borrows from
// `Hebi`, meaning that `Hebi` may not be moved (potentially to another thread)
// while that value is borrowed.
unsafe impl Send for Hebi {}

impl Hebi {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn check(&self, src: &str) -> Result<(), Vec<syntax::Error>> {
    syntax::parse(src)?;
    Ok(())
  }

  pub fn eval<'a, T: FromHebi<'a>>(&'a self, src: &str) -> Result<T> {
    let ctx = self.isolate.borrow().ctx();
    let module = syntax::parse(src)?;
    let module = emit::emit(ctx.clone(), "code", &module, true).unwrap();
    let module = module.instance(&ctx, None);
    let result = self.isolate.borrow_mut().run(module.root())?;
    let result = Value::bind(result);
    let ctx = self.ctx();
    T::from_hebi(&ctx, result)
  }

  pub fn io<T: 'static>(&self) -> Option<Ref<'_, T>> {
    match Ref::filter_map(self.isolate.borrow(), |isolate| {
      isolate.io().as_any().downcast_ref()
    }) {
      Ok(v) => Some(v),
      _ => None,
    }
  }

  pub fn globals(&self) -> Globals {
    Globals { hebi: self }
  }

  pub(crate) fn ctx(&self) -> public::Context {
    public::Context::bind(self.isolate.borrow().ctx())
  }
}

impl Hebi {
  /// Wraps a `T` in a native class instance.
  ///
  /// # Panics
  ///
  /// If `T` has not been registered to this `Hebi` instance via `globals`.
  pub fn wrap<T: TypeInfo + 'static>(&self, v: T) -> Value<'_> {
    self.try_wrap(v).unwrap()
  }

  /// Attempts to wrap a `T` in a native class instance.
  ///
  /// Fails if `T` has not been registered to this `Hebi` instance via
  /// `globals`.
  pub fn try_wrap<T: TypeInfo + 'static>(&self, v: T) -> Result<Value<'_>> {
    let class = {
      let classes = self.classes.borrow();
      let Some(class) = classes.get(&any::TypeId::of::<T>()) else {
        return Err(Error::runtime(format!(
          "`{}` has not been registered in this Hebi instance yet, use `Hebi::globals()` to register it",
          any::type_name::<T>()
        )));
      };
      class.clone()
    };
    let ctx = self.ctx();
    Ok(Value::bind(NativeClassInstance::new(
      ctx.inner(),
      class,
      UserData::new(ctx.inner(), v),
    )))
  }
}

pub struct Globals<'a> {
  hebi: &'a Hebi,
}

impl<'a> Globals<'a> {
  pub fn get(&self, name: &str) -> Option<Value<'a>> {
    self.hebi.isolate.borrow().get_global(name).map(Value::bind)
  }

  pub fn set(&mut self, name: impl IntoStr<'a>, value: Value<'a>) {
    let ctx = self.hebi.ctx();
    let name = name.into_str(&ctx);
    self
      .hebi
      .isolate
      .borrow_mut()
      .set_global(name.unbind(), value.unbind());
  }

  pub fn register_fn(&mut self, name: impl IntoStr<'a>, f: impl Function + 'static) {
    let ctx = self.hebi.ctx();
    let name = name.into_str(&ctx);
    self.set(
      name.clone(),
      Value::bind(NativeFunction::new(ctx.inner(), name.unbind(), Box::new(f))),
    )
  }

  pub fn register_class<T: TypeInfo + 'static>(&mut self) {
    let ctx = self.hebi.ctx();
    let class = NativeClass::new::<T>(ctx.inner());
    self
      .hebi
      .classes
      .borrow_mut()
      .insert(any::TypeId::of::<T>(), class.clone());
    self.set(class.name(), Value::bind(class))
  }
}

pub struct HebiBuilder {
  stdout: Option<Box<dyn Stdout>>,
  module_loader: Option<Box<dyn ModuleLoader>>,
  use_builtins: bool,
}

impl Hebi {
  pub fn builder() -> HebiBuilder {
    HebiBuilder {
      stdout: None,
      module_loader: None,
      use_builtins: false,
    }
  }
}

impl HebiBuilder {
  pub fn with_io<T: Stdout + 'static>(mut self, stdout: T) -> Self {
    let _ = self.stdout.replace(Box::new(stdout));
    self
  }

  pub fn with_module_loader<T: ModuleLoader + 'static>(mut self, loader: T) -> Self {
    let _ = self.module_loader.replace(Box::new(loader));
    self
  }

  pub fn with_builtins(mut self) -> Self {
    self.use_builtins = true;
    self
  }

  pub fn build(mut self) -> Hebi {
    let ctx = Context::new();
    let stdout = self
      .stdout
      .take()
      .unwrap_or_else(|| Box::new(std::io::stdout()));
    let module_loader = self
      .module_loader
      .take()
      .unwrap_or_else(|| Box::new(NoopModuleLoader));
    let isolate = Isolate::new(ctx, stdout, module_loader);

    let vm = Hebi {
      isolate: RefCell::new(isolate),
      classes: RefCell::new(IndexMap::new()),
    };

    if self.use_builtins {
      builtins::register(&vm);
    }

    vm
  }
}

impl Default for Hebi {
  fn default() -> Self {
    Self::builder().with_builtins().build()
  }
}

/// The noop module loader refuses to load any modules.
pub struct NoopModuleLoader;
#[derive(Debug)]
pub struct ModuleLoadError {
  pub path: String,
}
impl Display for ModuleLoadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "could not load module `{}`", self.path)
  }
}
impl std::error::Error for ModuleLoadError {}
impl ModuleLoader for NoopModuleLoader {
  fn load(
    &mut self,
    path: &[String],
  ) -> std::result::Result<&str, Box<dyn std::error::Error + 'static>> {
    Err(Box::new(ModuleLoadError {
      path: format!("could not load module `{}`", path.join(".")),
    }))
  }
}

#[cfg(test)]
mod tests;
