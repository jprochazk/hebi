#![allow(clippy::new_without_default)]

pub mod dispatch;
pub mod global;
pub mod thread;

use std::fmt::Debug;
use std::future::Future;
use std::ptr::NonNull;

use beef::lean::Cow;
use global::Global;
use module::Module;

use self::global::{Input, Output};
use self::thread::{Stack, Thread};
use super::error::{Error, Result};
use super::object::function::Disassembly;
use super::object::module::{ModuleId, ModuleLoader};
use super::object::{builtin, module, Any, Function, List, Ptr, Str};
use super::value::Value;
use super::{codegen, syntax};
use crate::public::NativeModule;
use crate::span::SpannedError;

pub struct Vm {
  pub(crate) global: Global,
  pub(crate) root: Thread,
  pub(crate) stack: NonNull<Stack>,
}

impl Debug for Vm {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Vm")
      .field("global", &self.global)
      .field("root", &self.root)
      .finish()
  }
}

struct DefaultModuleLoader {}

impl module::ModuleLoader for DefaultModuleLoader {
  // TODO: return user error
  fn load(&self, path: &str) -> Result<Cow<'static, str>> {
    Err(Error::Vm(SpannedError::new(
      format!("failed to load module {path}"),
      0..0,
    )))
  }
}

pub struct Config {
  pub module_loader: Option<Box<dyn ModuleLoader>>,
  pub input: Option<Box<dyn Input>>,
  pub output: Option<Box<dyn Output>>,
}

impl Config {
  fn resolve(self) -> (Box<dyn ModuleLoader>, Box<dyn Input>, Box<dyn Output>) {
    (
      self
        .module_loader
        .unwrap_or_else(|| Box::new(DefaultModuleLoader {})),
      self.input.unwrap_or_else(|| Box::new(std::io::stdin())),
      self.output.unwrap_or_else(|| Box::new(std::io::stdout())),
    )
  }
}

impl Default for Config {
  fn default() -> Self {
    Self {
      module_loader: Some(Box::new(DefaultModuleLoader {})),
      input: Some(Box::new(std::io::stdin())),
      output: Some(Box::new(std::io::stdout())),
    }
  }
}

impl Default for Vm {
  fn default() -> Self {
    Self::with_config(Config::default())
  }
}

impl Vm {
  pub fn with_config(config: Config) -> Self {
    let global = Global::new(config);
    builtin::register_builtin_functions(&global);
    let stack = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Stack::new()))) };
    let root = Thread::new(global.clone(), stack);
    Self {
      global,
      root,
      stack,
    }
  }

  pub async fn eval(&mut self, code: &str) -> Result<Value> {
    let chunk = self.compile(code)?;
    self.entry(chunk).await
  }

  pub fn compile(&self, code: &str) -> Result<Chunk> {
    let ast = syntax::parse(self.global.clone(), code).map_err(Error::Syntax)?;
    let module = codegen::emit(self.global.clone(), &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = self.global.alloc(List::new());
    let main = module.root.clone();
    let main = self.global.alloc(Function::new(main, upvalues, module_id));

    Ok(Chunk { main })
  }

  pub async fn entry(&mut self, chunk: Chunk) -> Result<Value> {
    self.root.entry(chunk.main).await
  }

  pub fn call<'a>(
    &'a mut self,
    callable: Ptr<Any>,
    args: &'a [Value],
  ) -> impl Future<Output = Result<Value>> + 'a {
    self.root.call(callable, args)
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

#[derive(Clone)]
pub struct Chunk {
  main: Ptr<Function>,
}

impl Chunk {
  pub fn disassemble(&self) -> Disassembly {
    self.main.descriptor.disassemble()
  }
}

impl Drop for Vm {
  fn drop(&mut self) {
    let _ = unsafe { Box::from_raw(self.stack.as_ptr()) };
  }
}

#[cfg(test)]
mod tests;
