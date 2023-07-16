use bumpalo::collections::Vec;

use super::Result;
use crate::lex::Span;
use crate::obj::func::Constant;
use crate::op::Op;
use crate::Arena;

// TODO: handle spans

pub struct BytecodeBuilder<'arena> {
  code: Vec<'arena, Op>,
  pool: Vec<'arena, Constant>,
  spans: Vec<'arena, Span>,
}

impl<'arena> BytecodeBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      code: Vec::new_in(arena),
      pool: Vec::new_in(arena),
      spans: Vec::new_in(arena),
    }
  }

  pub fn emit(&mut self, op: Op, span: impl Into<Span>) -> Result<()> {
    self.code.try_reserve(1)?;
    self.spans.try_reserve(1)?;

    self.code.push(op);
    self.spans.push(span.into());
    Ok(())
  }

  pub fn finish(mut self) -> (Vec<'arena, Op>, Vec<'arena, Constant>, Vec<'arena, Span>) {
    (self.code, self.pool, self.spans)
  }
}
