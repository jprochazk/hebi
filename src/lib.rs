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

/*

TODO: carefully design the public API
- Value
  - constructors
  - as_*
- Isolate
  - call
  - ?
*/

// TODO: everything that allocates should go through the context,
// eventually the context `alloc` method will use a faster allocator
// together with a garbage collector to make it worth the effort

use std::cell::{Ref, RefCell};
use std::fmt::{Debug, Display};

pub use error::Error;
use isolate::{Isolate, Stdout};
use public::IntoStr;
use value::Value as CoreValue;

pub type Result<T, E = Error> = std::result::Result<T, E>;

use ctx::Context;
pub use derive::function;
pub use public::conv::{FromHebi, IntoHebi};
pub use public::Value;
pub use value::object::module::ModuleLoader;
use value::object::native::Callable;
use value::object::NativeFunction;

pub struct Hebi {
  isolate: RefCell<Isolate>,
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
  #[deprecated = "use `Hebi::builder` or `Hebi::default` instead"]
  pub fn new() -> Self {
    Self::default()
  }

  #[deprecated = "use `Hebi::builder` instead"]
  pub fn with_io(io: impl Stdout) -> Self {
    Self::builder().with_io(io).build()
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
    let result = self
      .isolate
      .borrow_mut()
      .call(module.root().into(), &[], CoreValue::none())?;
    let result = Value::bind(result);
    let ctx = self.ctx();
    Ok(T::from_hebi(&ctx, result)?)
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
  pub fn create_function(&self, f: impl Callable + 'static) -> Value {
    Value::bind(
      self
        .isolate
        .borrow_mut()
        .alloc(NativeFunction::new(Box::new(f))),
    )
  }
}

pub struct Globals<'a> {
  hebi: &'a Hebi,
}

impl<'a> Globals<'a> {
  pub fn get(&self, name: &str) -> Option<Value<'a>> {
    self.hebi.isolate.borrow().get_global(name).map(Value::bind)
  }

  pub fn set(&mut self, name: impl IntoStr, value: Value<'a>) {
    let name = name.into_str(&self.hebi.ctx());
    self
      .hebi
      .isolate
      .borrow_mut()
      .set_global(name.unbind(), value.unbind());
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
