use bumpalo::collections::Vec;

use crate::op::Op;
use crate::Arena;

// TODO: handle spans

pub struct BytecodeBuilder<'arena> {
  code: Vec<'arena, Op>,
}

impl<'arena> BytecodeBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      code: Vec::new_in(arena),
    }
  }

  pub fn emit(&mut self, op: Op) {
    self.code.push(op);
  }
}
