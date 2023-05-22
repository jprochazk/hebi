#![allow(clippy::new_without_default)]

pub mod dispatch;
pub mod global;
pub mod thread;

use std::ptr::NonNull;

use global::Global;
use module::Module;

use self::global::{Input, Output};
use self::thread::{Stack, Thread};
use crate as hebi;
use crate::module::NativeModule;
use crate::object::function::Disassembly;
use crate::object::module::ModuleId;
use crate::object::{module, Function, List, Ptr, Str};
use crate::span::SpannedError;
use crate::value::Value;
use crate::{codegen, syntax, Error, ModuleLoader};

pub struct Vm {
  pub(crate) global: Global,
  pub(crate) root: Thread,
  pub(crate) stack: NonNull<Stack>,
}

struct DefaultModuleLoader {}

impl module::ModuleLoader for DefaultModuleLoader {
  // TODO: return user error
  fn load(&self, path: &str) -> hebi::Result<&str> {
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
    let stack = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(Stack::new()))) };
    let root = Thread::new(global.clone(), stack);
    Self {
      global,
      root,
      stack,
    }
  }

  pub async fn eval(&mut self, code: &str) -> hebi::Result<Value> {
    let chunk = self.compile(code)?;
    self.run(chunk).await
  }

  pub fn compile(&self, code: &str) -> hebi::Result<Chunk> {
    let ast = syntax::parse(self.global.clone(), code).map_err(Error::Syntax)?;
    let module = codegen::emit(self.global.clone(), &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = self.global.alloc(List::new());
    let main = module.root.clone();
    let main = self.global.alloc(Function::new(main, upvalues, module_id));

    Ok(Chunk { main })
  }

  pub async fn run(&mut self, chunk: Chunk) -> hebi::Result<Value> {
    self.root.call(Value::object(chunk.main.clone()), &[]).await
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
