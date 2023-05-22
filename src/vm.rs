#![allow(clippy::new_without_default)]

pub mod dispatch;
pub mod global;
pub mod thread;

use std::ptr::NonNull;

use global::Global;
use module::Module;

use self::thread::{Stack, Thread};
use crate as hebi;
use crate::module::NativeModule;
use crate::object::module::ModuleId;
use crate::object::{module, Function, List, Str};
use crate::span::SpannedError;
use crate::value::Value;
use crate::{emit, syntax, Error};

pub struct Vm {
  pub(crate) global: Global,
  pub(crate) root: Thread,
  pub(crate) stack: NonNull<Stack>,
}

struct DefaultModuleLoader {}

impl module::Loader for DefaultModuleLoader {
  // TODO: return user error
  fn load(&self, path: &str) -> hebi::Result<&str> {
    Err(Error::Vm(SpannedError::new(
      format!("failed to load module {path}"),
      0..0,
    )))
  }
}

impl Vm {
  pub fn new() -> Self {
    Self::with_module_loader(DefaultModuleLoader {})
  }

  pub fn with_module_loader(module_loader: impl module::Loader + 'static) -> Self {
    let global = Global::new(module_loader);
    let stack = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Stack::new()))) };
    let root = Thread::new(global.clone(), stack);
    Self {
      global,
      root,
      stack,
    }
  }

  pub async fn eval(&mut self, code: &str) -> hebi::Result<Value> {
    let ast = syntax::parse(self.global.clone(), code).map_err(Error::Syntax)?;
    let module = emit::emit(self.global.clone(), &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = self.global.alloc(List::new());
    let main = module.root.clone();
    let main = self.global.alloc(Function::new(main, upvalues, module_id));
    println!("{}", main.descriptor.disassemble());
    let main = Value::object(main);

    self.root.call(main, &[]).await
  }

  pub fn register(&mut self, module: &NativeModule) {
    let name = self.global.alloc(Str::owned(module.data.name.clone()));
    let module_id = self.root.global.next_module_id();
    let module = self.global.alloc(Module::native(
      self.global.clone(),
      name.clone(),
      module,
      module_id,
    ));
    self.root.global.define_module(module_id, name, module);
    self.root.global.finish_module(module_id, true);
  }
}

impl Drop for Vm {
  fn drop(&mut self) {
    let _ = unsafe { Box::from_raw(self.stack.as_ptr()) };
  }
}

#[cfg(test)]
mod tests;
