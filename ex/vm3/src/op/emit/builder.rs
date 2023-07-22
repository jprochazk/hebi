use core::hash::BuildHasherDefault;

use bumpalo::collections::Vec;

use super::{EmitError, HashMap, Result};
use crate::gc::Ref;
use crate::lex::Span;
use crate::obj::string::Str;
use crate::op::{Const, Op};
use crate::val::{Constant, NFloat};
use crate::Arena;

// TODO: handle spans

pub struct BytecodeBuilder<'arena> {
  code: Vec<'arena, Op>,
  pool: ConstantPoolBuilder<'arena>,
  spans: Vec<'arena, Span>,
}

impl<'arena> BytecodeBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      code: Vec::new_in(arena),
      pool: ConstantPoolBuilder::new_in(arena),
      spans: Vec::new_in(arena),
    }
  }

  #[inline]
  pub fn pool(&mut self) -> &mut ConstantPoolBuilder<'arena> {
    &mut self.pool
  }

  pub fn emit(&mut self, op: Op, span: impl Into<Span>) -> Result<()> {
    self.code.try_reserve(1)?;
    self.spans.try_reserve(1)?;

    self.code.push(op);
    self.spans.push(span.into());
    Ok(())
  }

  pub fn finish(self) -> (Vec<'arena, Op>, Vec<'arena, Constant>, Vec<'arena, Span>) {
    (self.code, self.pool.entries, self.spans)
  }
}

pub struct ConstantPoolBuilder<'arena> {
  entries: Vec<'arena, Constant>,
  float_map: HashMap<NFloat, Const<u16>, &'arena Arena>,
  str_map: HashMap<Ref<Str>, Const<u16>, &'arena Arena>,
}

impl<'arena> ConstantPoolBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      entries: Vec::new_in(arena),
      float_map: HashMap::with_hasher_in(BuildHasherDefault::default(), arena),
      str_map: HashMap::with_hasher_in(BuildHasherDefault::default(), arena),
    }
  }

  fn insert(&mut self, entry: Constant) -> Result<Const<u16>> {
    let idx = self.entries.len();
    if idx > u16::MAX as usize {
      return Err(EmitError::new(format!(
        "exceeded maximum number of constants ({})",
        u16::MAX
      )));
    }
    self.entries.push(entry);
    Ok(Const(idx as u16))
  }

  pub fn str(&mut self, v: Ref<Str>) -> Result<Const<u16>> {
    if let Some(idx) = self.str_map.get(&v).copied() {
      return Ok(idx);
    }
    self.insert(v.into())
  }

  pub fn float(&mut self, v: f64) -> Result<Const<u16>> {
    // Should never fail, because all floats created at compile time
    // are guaranteed to not be `NaN`.
    let v = NFloat::try_from(v).map_err(|()| EmitError::new(format!("invalid float: {v}")))?;
    if let Some(idx) = self.float_map.get(&v).copied() {
      return Ok(idx);
    }
    self.insert(v.into())
  }
}
