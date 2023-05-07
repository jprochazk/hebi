use std::rc::Rc;

use crate::ctx::Context;
use crate::object::{Ptr, Table};

#[derive(Clone)]
pub struct Global(Rc<State>);

struct State {
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
}
