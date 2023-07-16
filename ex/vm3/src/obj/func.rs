use core::fmt::{Debug, Display};
use core::ptr::NonNull;

use bumpalo::AllocErr;

use super::string::Str;
use crate::gc::{Gc, Object, Ref};
use crate::lex::Span;
use crate::op::Op;

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

// TODO:
pub enum Constant {}

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
    Op::LoadConst { dst, idx } => write!(f, "load_const {idx}, {dst}")?,
    Op::LoadUpvalue { dst, idx } => write!(f, "load_upvalue {idx}, {dst}")?,
    Op::SetUpvalue { src, idx } => write!(f, "set_upvalue {idx}, {src}")?,
    Op::LoadMvar { dst, idx } => write!(f, "load_mvar {idx}, {dst}")?,
    Op::SetMvar { src, idx } => write!(f, "set_mvar {idx}, {src}")?,
    Op::LoadGlobal { dst, name } => write!(f, "load_global {name}, {dst}")?,
    Op::SetGlobal { src, name } => write!(f, "set_global {name}, {src}")?,
    Op::LoadFieldReg { obj, name, dst } => write!(f, "load_field {obj}, {name}, {dst}")?,
    Op::LoadFieldConst { obj, name, dst } => write!(f, "load_field {obj}, {name}, {dst}")?,
    Op::LoadFieldOptReg { obj, name, dst } => write!(f, "load_field? {obj}, {name}, {dst}")?,
    Op::LoadFieldOptConst { obj, name, dst } => write!(f, "load_field? {obj}, {name}, {dst}")?,
    Op::SetField { obj, name, src } => write!(f, "set_field {obj}, {name}, {src}")?,
    Op::LoadIndex { obj, key, dst } => write!(f, "load_index {obj}, {key}, {dst}")?,
    Op::LoadIndexOpt { obj, key, dst } => write!(f, "load_index? {obj}, {key}, {dst}")?,
    Op::SetIndex { obj, key, src } => write!(f, "set_index {obj}, {key}, {src}")?,
    Op::LoadSuper { dst } => todo!(),
    Op::LoadNil { dst } => write!(f, "load_nil {dst}")?,
    Op::LoadTrue { dst } => todo!(),
    Op::LoadFalse { dst } => todo!(),
    Op::LoadSmi { dst, value } => write!(f, "load_smi {value}, {dst}")?,
    Op::MakeFn { dst, desc } => todo!(),
    Op::MakeClass { dst, desc } => todo!(),
    Op::MakeClassDerived { dst, desc } => todo!(),
    Op::MakeList { dst, desc } => todo!(),
    Op::MakeListEmpty { dst } => todo!(),
    Op::MakeTable { dst, desc } => todo!(),
    Op::MakeTableEmpty { dst } => todo!(),
    Op::Jump { offset } => todo!(),
    Op::JumpConst { offset } => todo!(),
    Op::JumpLoop { offset } => todo!(),
    Op::JumpLoopConst { offset } => todo!(),
    Op::JumpIfFalse { offset } => todo!(),
    Op::JumpIfFalseConst { offset } => todo!(),
    Op::Add { dst, lhs, rhs } => todo!(),
    Op::Sub { dst, lhs, rhs } => todo!(),
    Op::Mul { dst, lhs, rhs } => todo!(),
    Op::Div { dst, lhs, rhs } => todo!(),
    Op::Rem { dst, lhs, rhs } => todo!(),
    Op::Pow { dst, lhs, rhs } => todo!(),
    Op::Inv { val } => todo!(),
    Op::Not { val } => todo!(),
    Op::CmpEq { lhs, rhs } => todo!(),
    Op::CmpNe { lhs, rhs } => todo!(),
    Op::CmpGt { lhs, rhs } => todo!(),
    Op::CmpGe { lhs, rhs } => todo!(),
    Op::CmpLt { lhs, rhs } => todo!(),
    Op::CmpLe { lhs, rhs } => todo!(),
    Op::CmpType { lhs, rhs } => todo!(),
    Op::Contains { lhs, rhs } => todo!(),
    Op::IsNone { val } => todo!(),
    Op::Call { func, count } => todo!(),
    Op::Call0 { func } => todo!(),
    Op::Import { dst, path } => todo!(),
    Op::FinalizeModule => write!(f, "finalize_module")?,
    Op::Ret { val } => write!(f, "ret {val}")?,
    Op::Yld { val } => todo!(),
  }

  // TODO: write code using `loc`, updating `prev_loc` each time
  let _ = loc;
  // TODO: emit + store + print labels
  /* if !loc.is_empty() && loc != &prev_loc {
    write!(f, " ; {}", )
  } */

  writeln!(f)
}
