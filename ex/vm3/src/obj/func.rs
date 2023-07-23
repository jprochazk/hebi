use core::fmt::{Debug, Display, Write};
use core::ptr::NonNull;

use bumpalo::AllocErr;

use super::string::Str;
use crate::gc::{Gc, Object, Ref};
use crate::lex::Span;
use crate::op::Op;
use crate::val::Constant;

pub struct FunctionDescriptor {
  name: Ref<Str>,
  params: Params,
  stack_space: u8,
  ops: NonNull<[Op]>,
  pool: NonNull<[Constant]>,
  dbg: DebugInfo,
}

pub struct DebugInfo {
  src: Ref<Str>,
  spans: NonNull<[Span]>,
  labels: NonNull<[(usize, LabelInfo)]>,
}

impl DebugInfo {
  pub fn src(&self) -> &str {
    self.src.as_str()
  }

  pub fn loc(&self) -> &[Span] {
    unsafe { self.spans.as_ref() }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct LabelInfo {
  pub name: &'static str,
  pub index: usize,
}

impl FunctionDescriptor {
  pub fn try_new_in(
    gc: &Gc,
    name: &str,
    params: Params,
    code: Code,
  ) -> Result<Ref<Self>, AllocErr> {
    let name = Str::try_new_in(gc, name)?;
    let ops = gc.try_alloc_slice(code.ops)?.into();
    let pool = gc.try_alloc_slice(code.pool)?.into();
    let spans = gc.try_alloc_slice(code.spans)?.into();
    let labels = gc.try_alloc_slice(code.labels)?.into();

    gc.try_alloc(FunctionDescriptor {
      name,
      params,
      stack_space: code.stack_space,
      ops,
      pool,
      dbg: DebugInfo {
        src: code.src,
        spans,
        labels,
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
}

impl Debug for FunctionDescriptor {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("FunctionDescriptor")
      .field("name", &self.name)
      .field("params", &self.params)
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
  pub spans: &'a [Span],
  pub labels: &'a [(usize, LabelInfo)],
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
    let labels = unsafe { func.dbg.labels.as_ref() };
    let ops = func.ops();
    let pool = func.pool();
    let loc = func.loc();

    writeln!(
      f,
      "function `{}` (registers: {}, length: {} ({} bytes))",
      func.name,
      func.stack_space(),
      func.ops.len(),
      func.ops.len() * 4,
    )?;

    // TODO: emit upvalues
    /* if !function.upvalues.borrow().is_empty() {
      writeln!(f, ".upvalues")?;
      for (index, upvalue) in function.upvalues.borrow().iter().enumerate() {
        match upvalue {
          Upvalue::Register(r) => writeln!(f, "  {index} <- {r}",)?,
          Upvalue::Upvalue(u) => writeln!(f, "  {index} <- {u}",)?,
        }
      }
    } */

    if !pool.is_empty() {
      writeln!(f, ".const")?;
      for (i, v) in pool.iter().enumerate() {
        writeln!(f, "  {i}: {v:?}")?;
      }
    }

    let mut label_iter = labels.iter().peekable();
    if !ops.is_empty() {
      writeln!(f, ".code")?;
      let mut prev_line_span = Span::empty();
      for (offset, (op, span)) in ops.iter().zip(loc.iter()).enumerate() {
        // write labels if one or more exists at the current offset
        while label_iter.peek().is_some_and(|(loff, _)| *loff == offset) {
          let (_, label) = label_iter.next().unwrap();
          writeln!(f, "<{}#{}>:", label.name, label.index)?;
        }
        let written = disasm_op(offset, op, labels, pool, f)?;
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

    // TODO: emit nested functions
    /* for v in pool {

    } */

    Ok(())
  }
}

fn disasm_op(
  base: usize,
  op: &Op,
  labels: &[(usize, LabelInfo)],
  pool: &[Constant],
  f: &mut core::fmt::Formatter,
) -> core::result::Result<usize, core::fmt::Error> {
  macro_rules! w {
    ($f:ident, $($tt:tt)*) => {{
      let mut proxy = ProxyFmt($f, 0);
      write!(&mut proxy, $($tt)*)?;
      Ok(proxy.written())
    }}
  }

  macro_rules! c {
    ($p:expr, $i:expr, $ty:ident) => {{
      match ($p)[$i.wide()] {
        crate::val::Constant::$ty(v) => v,
        _ => unreachable!(),
      }
    }};
  }

  macro_rules! label {
    ($l:expr, $op:tt, $b:expr, $o:expr) => {{
      let o = $b as u64 $op u64::from(($o).0);
      let (_, label) = labels.iter().find(|(oo, _)| (*oo) as u64 == o).unwrap();
      format_args!("{}#{}", label.name, label.index)
    }}
  }

  #[rustfmt::skip]
  let written = {
    match *op {
      Op::Nop =>                                  w!(f, "  nop"),
      Op::Mov { src, dst } =>                     w!(f, "  mov   {src}, {dst}"),
      Op::LoadConst { dst, idx } =>               w!(f, "  lc    {idx}, {dst}"),
      Op::LoadUpvalue { dst, idx } =>             w!(f, "  lup   {idx}, {dst}"),
      Op::SetUpvalue { src, idx } =>              w!(f, "  sup   {src}, {idx}"),
      Op::LoadMvar { dst, idx } =>                w!(f, "  lmv   {idx}, {dst}"),
      Op::SetMvar { src, idx } =>                 w!(f, "  smv   {src}, {idx}"),
      Op::LoadGlobal { dst, name } =>             w!(f, "  lg    {name}, {dst}"),
      Op::SetGlobal { src, name } =>              w!(f, "  sg    {src}, {name}"),
      Op::LoadFieldReg { obj, name, dst } =>      w!(f, "  ln    {obj}, {name}, {dst}"),
      Op::LoadFieldConst { obj, name, dst } =>    w!(f, "  ln    {obj}, {name}, {dst}"),
      Op::LoadFieldOptReg { obj, name, dst } =>   w!(f, "  ln?   {obj}, {name}, {dst}"),
      Op::LoadFieldOptConst { obj, name, dst } => w!(f, "  ln?   {obj}, {name}, {dst}"),
      Op::SetField { obj, name, src } =>          w!(f, "  sn    {src}, {obj}, {name}"),
      Op::LoadIndex { obj, key, dst } =>          w!(f, "  li    {obj}, {key}, {dst}"),
      Op::LoadIndexOpt { obj, key, dst } =>       w!(f, "  li?   {obj}, {key}, {dst}"),
      Op::SetIndex { obj, key, src } =>           w!(f, "  si    {src}, {obj}, {key}"),
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
      Op::Jump { offset } =>                      w!(f, "  jmp   {}", label!(labels, +, base, offset)),
      Op::JumpConst { offset } =>                 w!(f, "  jmp   {}", label!(labels, +, base, c!(pool, offset, Offset))),
      Op::JumpLoop { offset } =>                  w!(f, "  jl    {}", label!(labels, -, base, offset)),
      Op::JumpLoopConst { offset } =>             w!(f, "  jl    {}", label!(labels, -, base, c!(pool, offset, Offset))),
      Op::JumpIfFalse { val, offset } =>          w!(f, "  jf    {val}, {}", label!(labels, +, base, offset)),
      Op::JumpIfFalseConst { val, offset } =>     w!(f, "  jf    {val}, {}", label!(labels, +, base, c!(pool, offset, Offset))),
      Op::JumpIfTrue { val, offset } =>           w!(f, "  jt    {val}, {}", label!(labels, +, base, offset)),
      Op::JumpIfTrueConst { val, offset } =>      w!(f, "  jt    {val}, {}", label!(labels, +, base, c!(pool, offset, Offset))),
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
      Op::FinalizeModule =>                       w!(f, "  fin"),
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
