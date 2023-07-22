use core::hash::BuildHasherDefault;

use bumpalo::collections::Vec;

use super::{EmitError, HashMap, Result};
use crate::gc::Ref;
use crate::lex::Span;
use crate::obj::func::LabelInfo;
use crate::obj::list::ListDescriptor;
use crate::obj::string::Str;
use crate::obj::table::TableDescriptor;
use crate::obj::tuple::TupleDescriptor;
use crate::op::{Const, Op};
use crate::val::{Constant, NFloat};
use crate::Arena;

// TODO: handle spans

pub struct BytecodeBuilder<'arena> {
  code: Vec<'arena, Op>,
  pool: ConstantPoolBuilder<'arena>,
  spans: Vec<'arena, Span>,
  labels: Vec<'arena, (usize, LabelInfo)>,
}

impl<'arena> BytecodeBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      code: Vec::new_in(arena),
      pool: ConstantPoolBuilder::new_in(arena),
      spans: Vec::new_in(arena),
      labels: Vec::new_in(arena),
    }
  }

  #[inline]
  pub fn pool(&mut self) -> &mut ConstantPoolBuilder<'arena> {
    &mut self.pool
  }

  pub fn emit(&mut self, op: Op, span: impl Into<Span>) -> Result<()> {
    self.code.push(op);
    self.spans.push(span.into());
    Ok(())
  }

  #[allow(clippy::type_complexity)]
  pub fn finish(
    self,
  ) -> (
    Vec<'arena, Op>,
    Vec<'arena, Constant>,
    Vec<'arena, Span>,
    Vec<'arena, (usize, LabelInfo)>,
  ) {
    (self.code, self.pool.entries, self.spans, self.labels)
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

  pub fn float(&mut self, v: f64) -> Result<Const<u16>> {
    // Should never fail, because all floats created at compile time
    // are guaranteed to not be `NaN`.
    let v = NFloat::try_from(v).map_err(|()| EmitError::new(format!("invalid float: {v}")))?;
    if let Some(idx) = self.float_map.get(&v).copied() {
      return Ok(idx);
    }
    let idx = self.insert(v.into())?;
    self.float_map.insert_unique_unchecked(v, idx);
    Ok(idx)
  }

  pub fn str(&mut self, v: Ref<Str>) -> Result<Const<u16>> {
    if let Some(idx) = self.str_map.get(&v).copied() {
      return Ok(idx);
    }
    let idx = self.insert(v.into())?;
    self.str_map.insert_unique_unchecked(v, idx);
    Ok(idx)
  }

  pub fn table(&mut self, v: Ref<TableDescriptor>) -> Result<Const<u16>> {
    self.insert(v.into())
  }

  pub fn list(&mut self, v: Ref<ListDescriptor>) -> Result<Const<u16>> {
    self.insert(v.into())
  }

  pub fn tuple(&mut self, v: Ref<TupleDescriptor>) -> Result<Const<u16>> {
    self.insert(v.into())
  }
}
