use std::fmt::{Debug, Display};

use super::global::Global;
use crate::bytecode::opcode as op;
use crate::ctx::Context;
use crate::object::{List, Object, Ptr};
use crate::value::Value;

pub struct Thread {
  cx: Context,
  global: Global,

  // TODO: share stack when possible
  stack: Ptr<List>,
  acc: Value,
  width: op::Width,
  pc: usize,
}

impl Thread {
  pub fn new(cx: Context, global: Global) -> Self {
    Thread {
      cx: cx.clone(),
      global,

      stack: cx.alloc(List::with_capacity(0)),
      acc: Value::none(),
      width: op::Width::Normal,
      pc: 0,
    }
  }
}

impl super::Hebi {
  pub fn new_thread(&self) -> Thread {
    Thread::new(self.cx.clone(), self.global.clone())
  }
}

impl Display for Thread {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<thread>")
  }
}

impl Debug for Thread {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Thread")
      .field("stack", &self.stack)
      .field("acc", &self.acc)
      .field("width", &self.width)
      .field("pc", &self.pc)
      .finish()
  }
}

impl Object for Thread {
  fn type_name(&self) -> &'static str {
    "Thread"
  }
}
