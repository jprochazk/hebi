use core::fmt::{Debug, Display, Write};
use core::ptr::NonNull;

use beef::lean::Cow;
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump as Arena;

use super::string::Str;
use crate::ds::map::{BumpHashMap, GcHashMap, GcHashMapN};
use crate::ds::{fx, HasAlloc};
use crate::error::AllocError;
use crate::gc::{Alloc, Gc, Object, Ref};
use crate::lex::Span;
use crate::op::emit::CaptureInfo;
use crate::op::{Capture, Op, Reg};
use crate::val::Constant;

pub struct FunctionDescriptor {
  name: Ref<Str>,
  params: Params,
  stack_space: u8,
  captures: NonNull<[CaptureSource]>,
  ops: NonNull<[Op]>,
  pool: NonNull<[Constant]>,
  dbg: DebugInfo,
}

pub enum CaptureSource {
  NonLocal(Reg<u8>),
  Parent(Capture<u16>),
}

impl From<CaptureInfo> for CaptureSource {
  fn from(value: CaptureInfo) -> Self {
    match value {
      CaptureInfo::NonLocal { src, dst } => CaptureSource::NonLocal(src),
      CaptureInfo::Parent { src, dst } => CaptureSource::Parent(src),
    }
  }
}

pub struct DebugInfo {
  src: Ref<Str>,
  spans: NonNull<[Span]>,
  label_map: LabelMap,
}

impl DebugInfo {
  pub fn src(&self) -> &str {
    self.src.as_str()
  }

  pub fn loc(&self) -> &[Span] {
    unsafe { self.spans.as_ref() }
  }
}

pub struct LabelMapBuilder<'arena> {
  arena: &'arena Arena,
  labels: BumpVec<'arena, LabelInfo>,
  offset: BumpHashMap<'arena, usize, BumpVec<'arena, usize>>,
  referrer: BumpHashMap<'arena, usize, usize>,
}

impl<'arena> LabelMapBuilder<'arena> {
  pub fn new_in(arena: &'arena Arena) -> Self {
    Self {
      arena,
      labels: BumpVec::new_in(arena),
      offset: BumpHashMap::with_hasher_in(fx(), arena),
      referrer: BumpHashMap::with_hasher_in(fx(), arena),
    }
  }

  pub fn reserve_label(&mut self, name: &'static str) -> LabelInfo {
    let index = self.labels.len();
    let info = LabelInfo { name, index };
    self.labels.push(info);
    info
  }

  pub fn on_emit(&mut self, label: LabelInfo, referrer: usize) {
    self.referrer.insert(referrer, label.index);
  }

  pub fn on_bind(&mut self, label: LabelInfo, offset: usize) {
    let indices = self
      .offset
      .entry(offset)
      .or_insert_with(|| BumpVec::new_in(self.arena));
    indices.push(label.index);
  }

  pub fn finish(self, gc: &Gc) -> Result<LabelMap, AllocError> {
    let labels: NonNull<_> = gc.try_alloc_slice(self.labels.into_bump_slice())?.into();

    let mut offset = GcHashMap::with_hasher_in(fx(), Alloc::new(gc));
    offset.try_reserve(self.offset.len())?;
    for (o, indices) in self.offset.into_iter() {
      let indices: NonNull<_> = gc.try_alloc_slice(indices.into_bump_slice())?.into();
      offset.insert_unique_unchecked(o, indices);
    }
    let offset = offset.to_no_alloc();

    let mut referrer = GcHashMap::with_hasher_in(fx(), Alloc::new(gc));
    referrer.try_reserve(self.referrer.len())?;
    for (o, i) in self.referrer.into_iter() {
      referrer.insert_unique_unchecked(o, i);
    }
    let referrer = referrer.to_no_alloc();

    Ok(LabelMap {
      labels,
      offset,
      referrer,
    })
  }
}

pub struct LabelMap {
  labels: NonNull<[LabelInfo]>,
  offset: GcHashMapN<usize, NonNull<[usize]>>,
  referrer: GcHashMapN<usize, usize>,
}

impl LabelMap {
  pub fn by_offset(&self, offset: usize) -> Option<impl Iterator<Item = &LabelInfo> + '_> {
    self.offset.get(&offset).map(|indices| {
      let indices = unsafe { indices.as_ref() };
      let labels = unsafe { self.labels.as_ref() };
      indices.iter().map(|&i| &labels[i])
    })
  }

  pub fn by_referrer(&self, referrer: usize) -> Option<&LabelInfo> {
    self.referrer.get(&referrer).map(|&i| {
      let labels = unsafe { self.labels.as_ref() };
      &labels[i]
    })
  }
}

#[derive(Debug, Clone, Copy)]
pub struct LabelInfo {
  pub name: &'static str,
  index: usize,
}

impl FunctionDescriptor {
  pub fn try_new_in(
    gc: &Gc,
    name: &str,
    params: Params,
    code: Code,
  ) -> Result<Ref<Self>, AllocError> {
    let name = Str::try_new_in(gc, name)?;
    let ops = gc.try_alloc_slice(code.ops)?.into();
    let pool = gc.try_alloc_slice(code.pool)?.into();
    let spans = gc.try_alloc_slice(code.spans)?.into();
    let captures = gc
      .try_collect_slice(
        code
          .captures
          .iter()
          .map(|(_, c)| Ok(CaptureSource::from(*c))),
      )?
      .into();
    let label_map = code.label_map.finish(gc)?;

    gc.try_alloc(FunctionDescriptor {
      name,
      params,
      stack_space: code.stack_space,
      captures,
      ops,
      pool,
      dbg: DebugInfo {
        src: code.src,
        spans,
        label_map,
      },
    })
  }

  #[inline]
  pub fn name(&self) -> Ref<Str> {
    self.name
  }

  #[inline]
  pub fn params(&self) -> &Params {
    &self.params
  }

  #[inline]
  pub fn stack_space(&self) -> u8 {
    self.stack_space
  }

  #[inline]
  pub fn ops(&self) -> &[Op] {
    unsafe { self.ops.as_ref() }
  }

  #[inline]
  pub fn pool(&self) -> &[Constant] {
    unsafe { self.pool.as_ref() }
  }

  #[inline]
  pub fn loc(&self) -> &[Span] {
    unsafe { self.dbg.spans.as_ref() }
  }

  #[inline]
  pub fn dbg(&self) -> &DebugInfo {
    &self.dbg
  }

  #[inline]
  pub fn captures(&self) -> &[CaptureSource] {
    unsafe { self.captures.as_ref() }
  }
}

impl Debug for FunctionDescriptor {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("FunctionDescriptor")
      .field("name", &self.name)
      .field("params", &self.params.min)
      .finish_non_exhaustive()
  }
}

impl Display for FunctionDescriptor {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<function `{}`>", self.name)
  }
}

impl Object for FunctionDescriptor {
  const NEEDS_DROP: bool = false;
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Params {
  pub min: u8,
  pub max: u8,
  pub has_self: bool,
}

impl Params {
  pub fn empty() -> Self {
    Self {
      min: 0,
      max: 0,
      has_self: false,
    }
  }
}

pub struct Code<'a> {
  pub src: Ref<Str>,
  pub ops: &'a [Op],
  pub pool: &'a [Constant],
  pub captures: &'a [(Cow<'a, str>, CaptureInfo)],
  pub spans: &'a [Span],
  pub label_map: LabelMapBuilder<'a>,
  pub stack_space: u8,
}

impl FunctionDescriptor {
  pub fn dis(&self) -> Disasm<'_> {
    Disasm(self)
  }
}

pub struct Disasm<'a>(&'a FunctionDescriptor);

impl<'a> Display for Disasm<'a> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let func = self.0;

    let src = func.dbg.src.as_str();
    let label_map = &func.dbg.label_map;
    let ops = func.ops();
    let pool = func.pool();
    let loc = func.loc();
    let captures = func.captures();

    writeln!(
      f,
      "function `{}` (registers: {}, length: {} ({} bytes))",
      func.name,
      func.stack_space(),
      func.ops.len(),
      func.ops.len() * 4,
    )?;

    if !captures.is_empty() {
      writeln!(f, ".captures")?;
      for (index, src) in captures.iter().enumerate() {
        match src {
          CaptureSource::NonLocal(r) => writeln!(f, "  {index}: nonlocal {r}",)?,
          CaptureSource::Parent(u) => writeln!(f, "  {index}: parent {u}",)?,
        }
      }
    }

    if !pool.is_empty() {
      writeln!(f, ".const")?;
      for (i, v) in pool.iter().enumerate() {
        writeln!(f, "  {i}: {v:?}")?;
      }
    }

    if !ops.is_empty() {
      writeln!(f, ".code")?;
      let mut prev_line_span = Span::empty();
      for (offset, (op, span)) in ops.iter().zip(loc.iter()).enumerate() {
        // write labels if one or more exists at the current offset
        for label in label_map.by_offset(offset).into_iter().flatten() {
          writeln!(f, "<{}#{}>:", label.name, label.index)?;
        }
        let written = disasm_op(offset, op, label_map, pool, f)?;
        let padding = remainder_to(24, written);
        // write line at `span`
        let line_span = find_line(src, span);
        if !span.is_empty() && line_span != prev_line_span {
          write!(f, "{:padding$}; {}", "", src[line_span].trim())?;
          prev_line_span = line_span;
        }
        writeln!(f)?;
      }
    }

    for v in pool {
      if let Constant::Func(child) = v {
        writeln!(f, "\n{}", child.dis())?;
      }
    }

    Ok(())
  }
}

fn disasm_op(
  base: usize,
  op: &Op,
  labels: &LabelMap,
  _pool: &[Constant],
  f: &mut core::fmt::Formatter,
) -> core::result::Result<usize, core::fmt::Error> {
  macro_rules! w {
    ($f:ident, $($tt:tt)*) => {{
      let mut proxy = ProxyFmt($f, 0);
      write!(&mut proxy, $($tt)*)?;
      Ok(proxy.written())
    }}
  }

  /* macro_rules! c {
    ($p:expr, $i:expr, $ty:ident) => {{
      match ($p)[$i.wide()] {
        crate::val::Constant::$ty(v) => v,
        _ => unreachable!(),
      }
    }};
  } */

  macro_rules! label {
    ($l:expr, $b:expr) => {{
      let label = $l.by_referrer($b).unwrap();
      format_args!("{}#{}", label.name, label.index)
    }};
  }

  #[rustfmt::skip]
  let written = {
    match *op {
      Op::Nop =>                                  w!(f, "  nop   "),
      Op::Mov { src, dst } =>                     w!(f, "  mov   {src}, {dst}"),
      Op::LoadConst { dst, idx } =>               w!(f, "  lc    {idx}, {dst}"),
      Op::LoadCapture { dst, idx } =>             w!(f, "  lup   {idx}, {dst}"),
      Op::StoreCapture { src, idx } =>            w!(f, "  sup   {src}, {idx}"),
      Op::LoadMvar { dst, idx } =>                w!(f, "  lmv   {idx}, {dst}"),
      Op::StoreMvar { src, idx } =>               w!(f, "  smv   {src}, {idx}"),
      Op::LoadGlobal { dst, key } =>              w!(f, "  lg    {key}, {dst}"),
      Op::StoreGlobal { src, key } =>             w!(f, "  sg    {src}, {key}"),
      Op::LoadField { obj, key, dst } =>          w!(f, "  lf    {obj}, {key}, {dst}"),
      Op::LoadFieldR { obj, key, dst } =>         w!(f, "  lf    {obj}, {key}, {dst}"),
      Op::LoadFieldOpt { obj, key, dst } =>       w!(f, "  lf?   {obj}, {key}, {dst}"),
      Op::LoadFieldROpt { obj, key, dst } =>      w!(f, "  lf?   {obj}, {key}, {dst}"),
      Op::LoadFieldInt { obj, key, dst } =>       w!(f, "  lfi   {obj}, {key}, {dst}"),
      Op::LoadFieldIntR { obj, key, dst } =>      w!(f, "  lfi   {obj}, {key}, {dst}"),
      Op::LoadFieldIntOpt { obj, key, dst } =>    w!(f, "  lfi?  {obj}, {key}, {dst}"),
      Op::LoadFieldIntROpt { obj, key, dst } =>   w!(f, "  lfi?  {obj}, {key}, {dst}"),
      Op::StoreField { obj, key, src } =>         w!(f, "  sf    {obj}, {key}, {src}"),
      Op::StoreFieldR { obj, key, src } =>        w!(f, "  sf    {obj}, {key}, {src}"),
      Op::StoreFieldInt { obj, key, src } =>      w!(f, "  sfi   {obj}, {key}, {src}"),
      Op::StoreFieldIntR { obj, key, src } =>     w!(f, "  sfi   {obj}, {key}, {src}"),
      Op::LoadIndex { obj, key, dst } =>          w!(f, "  li    {obj}, {key}, {dst}"),
      Op::LoadIndexOpt { obj, key, dst } =>       w!(f, "  li?   {obj}, {key}, {dst}"),
      Op::StoreIndex { obj, key, src } =>         w!(f, "  si    {obj}, {key}, {src}"),
      Op::LoadSuper { dst } =>                    w!(f, "  lsup  {dst}"),
      Op::LoadNil { dst } =>                      w!(f, "  lnil  {dst}"),
      Op::LoadTrue { dst } =>                     w!(f, "  lbt   {dst}"),
      Op::LoadFalse { dst } =>                    w!(f, "  lbf   {dst}"),
      Op::LoadSmi { dst, value } =>               w!(f, "  lsmi  {value}, {dst}"),
      Op::MakeFn { dst, desc } =>                 w!(f, "  mfn   {desc}, {dst}"),
      Op::MakeClass { dst, desc } =>              w!(f, "  mcls  {desc}, {dst}"),
      Op::MakeClassDerived { dst, desc } =>       w!(f, "  mclsd {desc}, {dst}"),
      Op::MakeList { dst, desc } =>               w!(f, "  mlst  {desc}, {dst}"),
      Op::MakeListEmpty { dst } =>                w!(f, "  mlste {dst}"),
      Op::MakeTable { dst, desc } =>              w!(f, "  mtbl  {desc}, {dst}"),
      Op::MakeTableEmpty { dst } =>               w!(f, "  mtble {dst}"),
      Op::MakeTuple { dst, desc } =>              w!(f, "  mtup  {desc}, {dst}"),
      Op::MakeTupleEmpty { dst } =>               w!(f, "  mtupe {dst}"),
      Op::Jump { .. } =>                          w!(f, "  jmp   {}", label!(labels, base)),
      Op::JumpConst { .. } =>                     w!(f, "  jmp   {}", label!(labels, base)),
      Op::JumpLoop { .. } =>                      w!(f, "  jl    {}", label!(labels, base)),
      Op::JumpLoopConst { .. } =>                 w!(f, "  jl    {}", label!(labels, base)),
      Op::JumpIfFalse { val, .. } =>              w!(f, "  jf    {val}, {}", label!(labels, base)),
      Op::JumpIfFalseConst { val, .. } =>         w!(f, "  jf    {val}, {}", label!(labels, base)),
      Op::JumpIfTrue { val, .. } =>               w!(f, "  jt    {val}, {}", label!(labels, base)),
      Op::JumpIfTrueConst { val, .. } =>          w!(f, "  jt    {val}, {}", label!(labels, base)),
      Op::Add { dst, lhs, rhs } =>                w!(f, "  add   {lhs}, {rhs}, {dst}"),
      Op::Sub { dst, lhs, rhs } =>                w!(f, "  sub   {lhs}, {rhs}, {dst}"),
      Op::Mul { dst, lhs, rhs } =>                w!(f, "  mul   {lhs}, {rhs}, {dst}"),
      Op::Div { dst, lhs, rhs } =>                w!(f, "  div   {lhs}, {rhs}, {dst}"),
      Op::Rem { dst, lhs, rhs } =>                w!(f, "  rem   {lhs}, {rhs}, {dst}"),
      Op::Pow { dst, lhs, rhs } =>                w!(f, "  pow   {lhs}, {rhs}, {dst}"),
      Op::Inv { val } =>                          w!(f, "  inv   {val}"),
      Op::Not { val } =>                          w!(f, "  not   {val}"),
      Op::CmpEq { dst, lhs, rhs } =>              w!(f, "  ceq   {lhs}, {rhs}, {dst}"),
      Op::CmpNe { dst, lhs, rhs } =>              w!(f, "  cne   {lhs}, {rhs}, {dst}"),
      Op::CmpGt { dst, lhs, rhs } =>              w!(f, "  cgt   {lhs}, {rhs}, {dst}"),
      Op::CmpGe { dst, lhs, rhs } =>              w!(f, "  cge   {lhs}, {rhs}, {dst}"),
      Op::CmpLt { dst, lhs, rhs } =>              w!(f, "  clt   {lhs}, {rhs}, {dst}"),
      Op::CmpLe { dst, lhs, rhs } =>              w!(f, "  cle   {lhs}, {rhs}, {dst}"),
      Op::CmpType { dst, lhs, rhs } =>            w!(f, "  cty   {lhs}, {rhs}, {dst}"),
      Op::Contains { dst, lhs, rhs } =>           w!(f, "  in    {lhs}, {rhs}, {dst}"),
      Op::IsNil { dst, val } =>                   w!(f, "  cn    {val}, {dst}"),
      Op::Call { func, count } =>                 w!(f, "  call  {func}, {count}"),
      Op::Call0 { func } =>                       w!(f, "  call  {func}, 0"),
      Op::Import { dst, path } =>                 w!(f, "  imp   {path}, {dst}"),
      Op::FinalizeModule =>                       w!(f, "  fin   "),
      Op::Ret { val } =>                          w!(f, "  ret   {val}"),
      Op::Yld { val } =>                          w!(f, "  yld   {val}"),
    }
  };
  written
}

fn find_line(src: &str, span: &Span) -> Span {
  let start = src[..span.start()].rfind('\n').unwrap_or(0);
  let end = src[span.end()..]
    .find('\n')
    .map(|v| v + span.end())
    .unwrap_or(src.len());
  Span {
    start: start as u32,
    end: end as u32,
  }
}

fn remainder_to(n: usize, v: usize) -> usize {
  if v < n {
    n - v
  } else {
    0
  }
}

struct ProxyFmt<'a>(&'a mut (dyn Write + 'a), usize);

impl<'a> Write for ProxyFmt<'a> {
  fn write_str(&mut self, s: &str) -> core::fmt::Result {
    self.1 += s.len();
    self.0.write_str(s)
  }
}

impl<'a> ProxyFmt<'a> {
  fn written(&self) -> usize {
    self.1
  }
}
