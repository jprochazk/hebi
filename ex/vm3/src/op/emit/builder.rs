use bumpalo::collections::Vec;

use crate::ds::fx;
use crate::ds::map::BumpHashMap;
use crate::error::{Error, Result};
use crate::gc::Ref;
use crate::lex::Span;
use crate::obj::func::{FunctionProto, LabelInfo, LabelMapBuilder};
use crate::obj::list::ListProto;
use crate::obj::map::MapProto;
use crate::obj::string::Str;
use crate::obj::tuple::TupleProto;
use crate::op::ux::u24;
use crate::op::{Const, Offset, Op};
use crate::val::{Constant, NFloat};
use crate::Arena;

pub struct BytecodeBuilder<'arena> {
  code: Vec<'arena, Op>,
  pool: ConstantPoolBuilder<'arena>,
  spans: Vec<'arena, Span>,
  label_map: LabelMapBuilder<'arena>,
}

impl<'arena> BytecodeBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      code: Vec::new_in(arena),
      pool: ConstantPoolBuilder::new_in(arena),
      spans: Vec::new_in(arena),
      label_map: LabelMapBuilder::new_in(arena),
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
    LabelMapBuilder<'arena>,
  ) {
    (self.code, self.pool.entries, self.spans, self.label_map)
  }
}

pub struct BasicLabel {
  info: LabelInfo,
  referrer_offset: Option<usize>,
}

impl BasicLabel {
  pub fn new(b: &mut BytecodeBuilder, name: &'static str) -> Self {
    Self {
      info: b.label_map.reserve_label(name),
      referrer_offset: None,
    }
  }

  pub fn emit<F>(&mut self, b: &mut BytecodeBuilder, op: F, span: Span) -> Result<()>
  where
    F: FnOnce() -> Op,
  {
    let referrer = b.code.len();
    b.label_map.on_emit(self.info, referrer);

    self.referrer_offset = Some(referrer);
    let op = op();
    debug_assert!(op.is_fwd_jump());
    b.emit(op, span)
  }

  pub fn bind(self, b: &mut BytecodeBuilder) -> Result<()> {
    let referrer_offset = self.referrer_offset.unwrap();
    patch_jump(referrer_offset, b)?;

    let offset = b.code.len();
    b.label_map.on_bind(self.info, offset);

    Ok(())
  }
}

pub struct MultiLabel<'arena> {
  info: LabelInfo,
  referrers: Vec<'arena, usize>,
}

impl<'arena> MultiLabel<'arena> {
  pub fn new(b: &mut BytecodeBuilder<'arena>, name: &'static str) -> Self {
    let arena = b.code.bump();
    Self {
      info: b.label_map.reserve_label(name),
      referrers: Vec::new_in(arena),
    }
  }

  pub fn emit<F>(&mut self, b: &mut BytecodeBuilder, op: F, span: Span) -> Result<()>
  where
    F: Fn() -> Op,
  {
    let referrer = b.code.len();
    b.label_map.on_emit(self.info, referrer);

    self.referrers.push(referrer);
    let op = op();
    debug_assert!(op.is_fwd_jump());
    b.emit(op, span)
  }

  pub fn bind(self, b: &mut BytecodeBuilder) -> Result<()> {
    for referrer in self.referrers.iter().copied() {
      patch_jump(referrer, b)?;
    }

    let offset = b.code.len();
    b.label_map.on_bind(self.info, offset);

    Ok(())
  }
}

pub struct LoopLabel {
  info: LabelInfo,
  offset: usize,
  bound: bool,
}

impl LoopLabel {
  pub fn new(b: &mut BytecodeBuilder, name: &'static str) -> Self {
    Self {
      info: b.label_map.reserve_label(name),
      offset: usize::MAX,
      bound: false,
    }
  }

  pub fn emit<F>(&self, b: &mut BytecodeBuilder, op: F, span: Span) -> Result<()>
  where
    F: Fn(JumpOffset) -> Op,
  {
    let referrer = b.code.len();
    b.label_map.on_emit(self.info, referrer);

    let offset = referrer - self.offset;
    let offset = match u24::try_from(offset)
      .map(Offset)
      .map_err(|_| Offset(offset as u64))
    {
      Ok(offset) => JumpOffset::Short(offset),
      Err(offset) => JumpOffset::Long(b.pool().offset(offset)?),
    };
    let op = op(offset);
    debug_assert!(op.is_bwd_jump());
    b.emit(op, span)
  }

  pub fn bind(&mut self, b: &mut BytecodeBuilder) {
    assert!(!self.bound);
    let offset = b.code.len();
    b.label_map.on_bind(self.info, offset);

    self.offset = offset;
    self.bound = true;
  }
}

pub enum JumpOffset {
  Short(Offset<u24>),
  Long(Const<u16>),
}

pub struct ConstantPoolBuilder<'arena> {
  entries: Vec<'arena, Constant>,
  float_map: BumpHashMap<'arena, NFloat, Const<u16>>,
  int_map: BumpHashMap<'arena, i32, Const<u16>>,
  str_map: BumpHashMap<'arena, Ref<Str>, Const<u16>>,
  offset_map: BumpHashMap<'arena, Offset<u64>, Const<u16>>,
}

impl<'arena> ConstantPoolBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      entries: Vec::new_in(arena),
      float_map: BumpHashMap::with_hasher_in(fx(), arena),
      int_map: BumpHashMap::with_hasher_in(fx(), arena),
      str_map: BumpHashMap::with_hasher_in(fx(), arena),
      offset_map: BumpHashMap::with_hasher_in(fx(), arena),
    }
  }

  #[inline]
  pub fn is_u8_index(&self) -> bool {
    self.entries.len() < u8::MAX as usize
  }

  fn insert(&mut self, entry: Constant) -> Result<Const<u16>> {
    let idx = self.entries.len();
    if idx > u16::MAX as usize {
      return Err(Error::simple(format!(
        "exceeded maximum number of constants ({})",
        u16::MAX
      )));
    }
    self.entries.push(entry);
    Ok(Const(idx as u16))
  }

  pub fn offset(&mut self, v: Offset<u64>) -> Result<Const<u16>> {
    if let Some(idx) = self.offset_map.get(&v).copied() {
      return Ok(idx);
    }
    let idx = self.insert(v.into())?;
    self.offset_map.insert_unique_unchecked(v, idx);
    Ok(idx)
  }

  pub fn float(&mut self, v: f64) -> Result<Const<u16>> {
    // Should never fail, because all floats created at compile time
    // are guaranteed to not be `NaN`.
    let v = NFloat::try_from(v).map_err(|()| Error::simple(format!("invalid float: {v}")))?;
    if let Some(idx) = self.float_map.get(&v).copied() {
      return Ok(idx);
    }
    let idx = self.insert(v.into())?;
    self.float_map.insert_unique_unchecked(v, idx);
    Ok(idx)
  }

  pub fn int(&mut self, v: i32) -> Result<Const<u16>> {
    if let Some(idx) = self.int_map.get(&v).copied() {
      return Ok(idx);
    }
    let idx = self.insert(v.into())?;
    self.int_map.insert_unique_unchecked(v, idx);
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

  pub fn map(&mut self, v: Ref<MapProto>) -> Result<Const<u16>> {
    self.insert(v.into())
  }

  pub fn list(&mut self, v: Ref<ListProto>) -> Result<Const<u16>> {
    self.insert(v.into())
  }

  pub fn tuple(&mut self, v: Ref<TupleProto>) -> Result<Const<u16>> {
    self.insert(v.into())
  }

  pub fn func(&mut self, v: Ref<FunctionProto>) -> Result<Const<u16>> {
    self.insert(v.into())
  }
}

fn patch_jump(referrer: usize, b: &mut BytecodeBuilder) -> Result<()> {
  let code = &mut b.code;
  let pool = &mut b.pool;

  let offset = code.len() - referrer;
  match code[referrer] {
    Op::Jump { .. } => match u24::try_from(offset).map(Offset) {
      Ok(offset) => code[referrer] = Op::Jump { offset },
      Err(_) => {
        let offset = pool.offset(Offset(offset as u64))?;
        code[referrer] = Op::JumpConst { idx: offset };
      }
    },
    Op::JumpIfFalse { val, .. } => match u16::try_from(offset).map(Offset) {
      Ok(offset) => code[referrer] = Op::JumpIfFalse { val, offset },
      Err(_) => {
        let offset = pool.offset(Offset(offset as u64))?;
        code[referrer] = Op::JumpIfFalseConst { val, idx: offset };
      }
    },
    Op::JumpIfTrue { val, .. } => match u16::try_from(offset).map(Offset) {
      Ok(offset) => code[referrer] = Op::JumpIfTrue { val, offset },
      Err(_) => {
        let offset = pool.offset(Offset(offset as u64))?;
        code[referrer] = Op::JumpIfTrueConst { val, idx: offset };
      }
    },
    op => {
      return Err(Error::simple(format!(
        "invalid instruction {op:?} at offset {referrer}, expected forward jump instruction"
      )))
    }
  };
  Ok(())
}
