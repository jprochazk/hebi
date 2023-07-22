use core::fmt::{Debug, Display};
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
  loc: NonNull<[Span]>,
}

impl FunctionDescriptor {
  pub fn try_new_in(
    gc: &Gc,
    name: &str,
    params: Params,
    code: Code,
  ) -> Result<Ref<Self>, AllocErr> {
    let Code {
      ops,
      pool,
      loc,
      stack_space,
    } = code;

    let name = Str::try_new_in(gc, name)?;
    let ops = gc.try_alloc_slice(ops)?.into();
    let pool = gc.try_alloc_slice(pool)?.into();
    let loc = gc.try_alloc_slice(loc)?.into();

    gc.try_alloc(FunctionDescriptor {
      name,
      params,
      stack_space,
      ops,
      pool,
      loc,
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
    unsafe { self.loc.as_ref() }
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
}

impl Params {
  pub fn empty() -> Self {
    Self { min: 0, max: 0 }
  }
}

pub struct Code<'a> {
  pub ops: &'a [Op],
  pub pool: &'a [Constant],
  pub loc: &'a [Span],
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

    let (ops, _, loc) = (func.ops(), func.pool(), func.loc());

    /* for constant in constants {
      match constant {
        Constant::Function(function) => {
          writeln!(f, "{}\n", function.disassemble())?;
        }
        Constant::Class(class) => {
          for method in class.methods.values() {
            writeln!(f, "{}\n", method.disassemble_as_method(class.name.clone()))?;
          }
        }
        _ => {}
      }
    } */

    writeln!(
      f,
      "function `{}` (registers: {}, length: {} ({} bytes))",
      func.name,
      func.stack_space(),
      ops.len(),
      ops.len() * 4,
    )?;
    /* if !function.upvalues.borrow().is_empty() {
      writeln!(f, ".upvalues")?;
      for (index, upvalue) in function.upvalues.borrow().iter().enumerate() {
        match upvalue {
          Upvalue::Register(r) => writeln!(f, "  {index} <- {r}",)?,
          Upvalue::Upvalue(u) => writeln!(f, "  {index} <- {u}",)?,
        }
      }
    } */
    writeln!(f, ".code")?;

    // let mut prev_loc = Span::empty();
    for (op, loc) in ops.iter().zip(loc.iter()) {
      disasm_op(op, loc, f)?;
    }

    Ok(())
  }
}

#[rustfmt::skip]
fn disasm_op(op: &Op, loc: &Span, f: &mut core::fmt::Formatter) -> core::fmt::Result {
  // TODO: print constants
  // TODO: finish the rest of this
  write!(f, "  ")?;
  match op {
    Op::Nop => write!(f, "nop")?,
    Op::Mov { src, dst } => write!(f, "mov {src}, {dst}")?,
    Op::LoadConst { dst, idx } => write!(f, "lc {idx}, {dst}")?,
    Op::LoadUpvalue { dst, idx } => write!(f, "lup {idx}, {dst}")?,
    Op::SetUpvalue { src, idx } => write!(f, "sup {src}, {idx}")?,
    Op::LoadMvar { dst, idx } => write!(f, "lmv {idx}, {dst}")?,
    Op::SetMvar { src, idx } => write!(f, "smv {src}, {idx}")?,
    Op::LoadGlobal { dst, name } => write!(f, "lg {name}, {dst}")?,
    Op::SetGlobal { src, name } => write!(f, "sg {src}, {name}")?,
    Op::LoadFieldReg { obj, name, dst } => write!(f, "ln {obj}, {name}, {dst}")?,
    Op::LoadFieldConst { obj, name, dst } => write!(f, "ln {obj}, {name}, {dst}")?,
    Op::LoadFieldOptReg { obj, name, dst } => write!(f, "ln? {obj}, {name}, {dst}")?,
    Op::LoadFieldOptConst { obj, name, dst } => write!(f, "ln? {obj}, {name}, {dst}")?,
    Op::SetField { obj, name, src } => write!(f, "sn {src}, {obj}, {name}")?,
    Op::LoadIndex { obj, key, dst } => write!(f, "li {obj}, {key}, {dst}")?,
    Op::LoadIndexOpt { obj, key, dst } => write!(f, "li? {obj}, {key}, {dst}")?,
    Op::SetIndex { obj, key, src } => write!(f, "si {src}, {obj}, {key}")?,
    Op::LoadSuper { dst } => write!(f, "lsup {dst}")?,
    Op::LoadNil { dst } => write!(f, "lnil {dst}")?,
    Op::LoadTrue { dst } => write!(f, "lbt {dst}")?,
    Op::LoadFalse { dst } => write!(f, "lbf {dst}")?,
    Op::LoadSmi { dst, value } => write!(f, "lsmi {value}, {dst}")?,
    Op::MakeFn { dst, desc } => write!(f, "mfn {desc}, {dst}")?,
    Op::MakeClass { dst, desc } => write!(f, "mcl {desc}, {dst}")?,
    Op::MakeClassDerived { dst, desc } => write!(f, "mcld {desc}, {dst}")?,
    Op::MakeList { dst, desc } => write!(f, "ml {desc}, {dst}")?,
    Op::MakeListEmpty { dst } => write!(f, "mle {dst}")?,
    Op::MakeTable { dst, desc } => write!(f, "mt {desc}, {dst}")?,
    Op::MakeTableEmpty { dst } => write!(f, "mte {dst}")?,
    Op::Jump { offset } => write!(f, "jmp {offset}")?,
    Op::JumpConst { offset } => write!(f, "jmp {offset}")?,
    Op::JumpLoop { offset } => write!(f, "jl {offset}")?,
    Op::JumpLoopConst { offset } => write!(f, "jl {offset}")?,
    Op::JumpIfFalse { offset } => write!(f, "jif {offset}")?,
    Op::JumpIfFalseConst { offset } => write!(f, "jif {offset}")?,
    Op::Add { dst, lhs, rhs } => write!(f, "add {lhs}, {rhs}, {dst}")?,
    Op::Sub { dst, lhs, rhs } => write!(f, "sub {lhs}, {rhs}, {dst}")?,
    Op::Mul { dst, lhs, rhs } => write!(f, "mul {lhs}, {rhs}, {dst}")?,
    Op::Div { dst, lhs, rhs } => write!(f, "div {lhs}, {rhs}, {dst}")?,
    Op::Rem { dst, lhs, rhs } => write!(f, "rem {lhs}, {rhs}, {dst}")?,
    Op::Pow { dst, lhs, rhs } => write!(f, "pow {lhs}, {rhs}, {dst}")?,
    Op::Inv { val } => write!(f, "inv {val}")?,
    Op::Not { val } => write!(f, "not {val}")?,
    Op::CmpEq { lhs, rhs } => write!(f, "ceq {lhs}, {rhs}")?,
    Op::CmpNe { lhs, rhs } => write!(f, "cne {lhs}, {rhs}")?,
    Op::CmpGt { lhs, rhs } => write!(f, "cgt {lhs}, {rhs}")?,
    Op::CmpGe { lhs, rhs } => write!(f, "cge {lhs}, {rhs}")?,
    Op::CmpLt { lhs, rhs } => write!(f, "clt {lhs}, {rhs}")?,
    Op::CmpLe { lhs, rhs } => write!(f, "cle {lhs}, {rhs}")?,
    Op::CmpType { lhs, rhs } => write!(f, "cty {lhs}, {rhs}")?,
    Op::Contains { lhs, rhs } => write!(f, "in {lhs}, {rhs}")?,
    Op::IsNone { val } => write!(f, "cn {val}")?,
    Op::Call { func, count } => write!(f, "call {func}, {count}")?,
    Op::Call0 { func } => write!(f, "call {func}, 0")?,
    Op::Import { dst, path } => write!(f, "imp {path}, {dst}")?,
    Op::FinalizeModule => write!(f, "fin")?,
    Op::Ret { val } => write!(f, "ret {val}")?,
    Op::Yld { val } => write!(f, "yld {val}")?,
  }

  // TODO: write code using `loc`, updating `prev_loc` each time
  let _ = loc;
  // TODO: emit + store + print labels
  /* if !loc.is_empty() && loc != &prev_loc {
    write!(f, " ; {}", )
  } */

  writeln!(f)
}
