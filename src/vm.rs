#![allow(clippy::new_without_default)]

mod dispatch;
mod global;
mod thread;

use std::cell::RefCell;
use std::fmt::Display;

use global::Global;

use self::thread::Thread;
use crate::ctx::Context;
use crate::error::Error;
use crate::object::module::ModuleId;
use crate::object::{module, Function, List};
use crate::util::JoinIter;
use crate::value::Value;
use crate::{emit, syntax};

pub struct Hebi {
  cx: Context,
  global: Global,
  root: RefCell<Thread>,
}

struct DefaultModuleLoader {}

impl module::Loader for DefaultModuleLoader {
  // TODO: return user error
  fn load(&self, _: &str) -> Option<&str> {
    None
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

  pub fn eval(&self, code: &str) -> EvalResult {
    let cx = &self.cx;
    let ast = syntax::parse(cx, code).map_err(EvalError::Parse)?;
    let module = emit::emit(cx, &ast, "__main__", true);
    let module_id = ModuleId::global();
    let upvalues = cx.alloc(List::new());
    let main = module.root.clone();
    let main = cx.alloc(Function::new(main, upvalues, module_id));
    println!("{}", main.descriptor.disassemble());
    let main = Value::object(main);

    let mut root = self.root.borrow_mut();
    root.call(main, &[]).map_err(EvalError::Run)
  }

  pub fn cx(&self) -> &Context {
    &self.cx
  }
}

pub type EvalResult = core::result::Result<Value, EvalError>;

#[derive(Debug)]
pub enum EvalError {
  Parse(Vec<Error>),
  Run(Error),
}

impl Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      EvalError::Parse(e) => {
        write!(f, "{}", e.iter().join("\n"))
      }
      EvalError::Run(e) => {
        write!(f, "{e}")
      }
    }
  }
}

#[cfg(test)]
mod tests;
