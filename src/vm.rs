#![allow(clippy::new_without_default)]

mod dispatch;
mod global;
mod thread;

use std::cell::RefCell;
use std::marker::PhantomData;

use global::Global;

use self::thread::Thread;
use crate as hebi;
use crate::ctx::Context;
use crate::object::module::ModuleId;
use crate::object::{module, Function, List, Ptr};
use crate::span::SpannedError;
use crate::value::Value;
use crate::{emit, syntax, Error};

pub struct Hebi {
  cx: Context,
  global: Global,
  root: RefCell<Thread>,
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

impl Hebi {
  pub fn new() -> Self {
    let cx = Context::default();
    let global = Global::new(&cx, DefaultModuleLoader {});
    let root = RefCell::new(Thread::new(cx.clone(), global.clone()));
    Self { global, cx, root }
  }

  pub fn set_module_loader(&self, module_loader: impl module::Loader + 'static) {
    self.global.set_module_loader(module_loader)
  }

  pub fn eval(&self, code: &str) -> hebi::Result<Value> {
    let cx = &self.cx;
    let ast = syntax::parse(cx, code).map_err(Error::Syntax)?;
    let module = emit::emit(cx, &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = cx.alloc(List::new());
    let main = module.root.clone();
    let main = cx.alloc(Function::new(main, upvalues, module_id));
    println!("{}", main.descriptor.disassemble());
    let main = Value::object(main);

    let mut root = self.root.borrow_mut();
    root.call(main, &[])
  }

  pub fn cx(&self) -> &Context {
    &self.cx
  }
}

pub struct Scope<'a> {
  pub(crate) cx: Context,
  pub(crate) stack: Ptr<List>,
  pub(crate) stack_base: usize,
  pub(crate) args_start: usize,
  pub(crate) args_count: usize,
  pub(crate) lifetime: PhantomData<&'a ()>,
}

#[cfg(test)]
mod tests;
