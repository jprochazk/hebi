#![allow(clippy::new_without_default)]

pub mod dispatch;
pub mod global;
pub mod thread;

use global::Global;
use module::Module;

use self::thread::Thread;
use crate as hebi;
use crate::ctx::Context;
use crate::module::NativeModule;
use crate::object::module::ModuleId;
use crate::object::{module, Function, List, String};
use crate::span::SpannedError;
use crate::value::Value;
use crate::{emit, syntax, Error};

pub struct Vm {
  pub(crate) cx: Context,
  pub(crate) root: Thread,
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
    let cx = Context::default();
    let global = Global::new(&cx, DefaultModuleLoader {});
    let root = Thread::new(cx.clone(), global);
    Self { cx, root }
  }

  pub fn eval(&mut self, code: &str) -> hebi::Result<Value> {
    let cx = &self.cx;
    let ast = syntax::parse(cx, code).map_err(Error::Syntax)?;
    let module = emit::emit(cx, &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = cx.alloc(List::new());
    let main = module.root.clone();
    let main = cx.alloc(Function::new(main, upvalues, module_id));
    println!("{}", main.descriptor.disassemble());
    let main = Value::object(main);

    self.root.call(main, &[])
  }

  pub async fn eval_async(&mut self, code: &str) -> hebi::Result<Value> {
    let cx = &self.cx;
    let ast = syntax::parse(cx, code).map_err(Error::Syntax)?;
    let module = emit::emit(cx, &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = cx.alloc(List::new());
    let main = module.root.clone();
    let main = cx.alloc(Function::new(main, upvalues, module_id));
    println!("{}", main.descriptor.disassemble());
    let main = Value::object(main);

    self.root.call_async(main, &[]).await
  }

  pub fn register(&mut self, module: &NativeModule) {
    let mut registry = self.root.global.module_registry_mut();
    let name = self.cx.alloc(String::owned(module.data.name.clone()));
    let module_id = registry.next_module_id();
    let module = self
      .cx
      .alloc(Module::native(&self.cx, name.clone(), module, module_id));
    registry.insert(module_id, name, module);
  }
}

#[cfg(test)]
mod tests;
