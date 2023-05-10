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
use crate::object::{Function, List};
use crate::util::JoinIter;
use crate::value::Value;
use crate::{emit, syntax};

pub struct Hebi {
  cx: Context,
  global: Global,
  root: RefCell<Thread>,
}

impl Hebi {
  pub fn new() -> Self {
    let cx = Context::default();
    let global = Global::new(&cx);
    let root = RefCell::new(Thread::new(cx.clone(), global.clone()));
    Self { global, cx, root }
  }

  pub fn eval(&self, code: &str) -> EvalResult {
    let ast = syntax::parse(&self.cx, code).map_err(EvalError::Parse)?;
    let module = emit::emit(&self.cx, &ast, "__main__", true);
    let module_root = self.cx.alloc(Function::new(
      &self.cx,
      module.root.clone(),
      self.cx.alloc(List::new()),
      ModuleId::null(),
    ));
    println!("{}", module_root.descriptor.disassemble());
    let value = self
      .root
      .borrow_mut()
      .call(Value::object(module_root), &[])
      .map_err(EvalError::Run)?;
    Ok(value)
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
