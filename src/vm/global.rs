use std::ops::Deref;
use std::rc::Rc;

use crate::ctx::Context;
use crate::object::{Ptr, Table};

#[derive(Clone)]
pub struct Global(Rc<State>);

pub struct State {
  globals: Ptr<Table>,
  // types: Types,
  // modules: Modules,
}

impl Global {
  pub fn new(cx: &Context) -> Self {
    Self(Rc::new(State {
      globals: cx.alloc(Table::with_capacity(0)),
    }))
  }

  pub fn globals(&self) -> &Ptr<Table> {
    &self.globals
  }
}

impl Deref for Global {
  type Target = State;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
