#![allow(clippy::new_without_default)]

mod dispatch;
mod global;
mod thread;

use global::Global;

use crate::ctx::Context;

pub struct Hebi {
  global: Global,

  cx: Context,
}

impl Hebi {
  pub fn new() -> Self {
    let cx = Context::default();

    Self {
      global: Global::new(&cx),

      cx,
    }
  }

  pub fn cx(&self) -> &Context {
    &self.cx
  }
}
