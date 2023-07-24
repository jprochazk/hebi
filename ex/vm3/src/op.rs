pub mod emit;

mod ux;
use core::fmt::Display;

use ux::u24;

use crate::util::static_assert_size;

/*
codegen notes:
- for constant indices stored as `u8`, the constant can
  first be loaded into a register, and then used, because
  `LoadConst` stores the constant index as `u16`, allowing
  a much greater range
*/

#[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Op {
  Nop,
  Mov { src: Reg<u8>, dst: Reg<u8> },
  LoadConst { dst: Reg<u8>, idx: Const<u16> },
  LoadUpvalue { dst: Reg<u8>, idx: Upvalue<u16> },
  StoreUpvalue { src: Reg<u8>, idx: Upvalue<u16> },
  LoadMvar { dst: Reg<u8>, idx: Mvar<u16> },
  StoreMvar { src: Reg<u8>, idx: Mvar<u16> },
  LoadGlobal { dst: Reg<u8>, key: Const<u16> },
  StoreGlobal { src: Reg<u8>, key: Const<u16> },
  LoadField { obj: Reg<u8>, key: Const<u8>, dst: Reg<u8> },
  LoadFieldR { obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8> },
  LoadFieldOpt { obj: Reg<u8>, key: Const<u8>, dst: Reg<u8> },
  LoadFieldROpt { obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8> },
  LoadFieldInt { obj: Reg<u8>, key: Const<u8>, dst: Reg<u8> },
  LoadFieldIntR { obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8> },
  LoadFieldIntOpt { obj: Reg<u8>, key: Const<u8>, dst: Reg<u8> },
  LoadFieldIntROpt { obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8> },
  // TODO: come up with a better scheme for storing fields
  // maybe an extra DATA op? that's probably necessary anyway for ICs
  StoreField { obj: Reg<u8>, key: Reg<u8>, src: Reg<u8> },
  LoadIndex { obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8> },
  LoadIndexOpt { obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8> },
  StoreIndex { obj: Reg<u8>, key: Reg<u8>, src: Reg<u8> },
  LoadSuper { dst: Reg<u8> },
  LoadNil { dst: Reg<u8> },
  LoadTrue { dst: Reg<u8> },
  LoadFalse { dst: Reg<u8> },
  LoadSmi { dst: Reg<u8>, value: Smi<i16> },
  MakeFn { dst: Reg<u8>, desc: Const<u16> },
  MakeClass { dst: Reg<u8>, desc: Const<u16> },
  MakeClassDerived { dst: Reg<u8>, desc: Const<u16> },
  MakeList { dst: Reg<u8>, desc: Const<u16> },
  MakeListEmpty { dst: Reg<u8> },
  MakeTable { dst: Reg<u8>, desc: Const<u16> },
  MakeTableEmpty { dst: Reg<u8> },
  MakeTuple { dst: Reg<u8>, desc: Const<u16> },
  MakeTupleEmpty { dst: Reg<u8> },
  Jump { offset: Offset<u24> },
  JumpConst { offset: Const<u16> },
  JumpLoop { offset: Offset<u24> },
  JumpLoopConst { offset: Const<u16> },
  JumpIfFalse { val: Reg<u8>, offset: Offset<u16> },
  JumpIfFalseConst { val: Reg<u8>, offset: Const<u16> },
  JumpIfTrue { val: Reg<u8>, offset: Offset<u16> },
  JumpIfTrueConst { val: Reg<u8>, offset: Const<u16> },
  Add { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Sub { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Mul { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Div { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Rem { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Pow { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Inv { val: Reg<u8> },
  Not { val: Reg<u8> },
  CmpEq { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  CmpNe { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  CmpGt { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  CmpGe { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  CmpLt { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  CmpLe { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  CmpType { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  Contains { dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8> },
  IsNil { dst: Reg<u8>, val: Reg<u8> },
  Call { func: Reg<u8>, count: Count<u8> },
  Call0 { func: Reg<u8> },
  Import { dst: Reg<u8>, path: Const<u16> },
  FinalizeModule,
  Ret { val: Reg<u8> },
  Yld { val: Reg<u8> },
}

const _: () = static_assert_size::<Op>(4, "expected a size of 4 bytes");

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reg<T>(pub T);

impl<T: Into<usize>> Reg<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

impl<T: Display> Display for Reg<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "r{}", self.0)
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Const<T>(pub T);

impl<T: Into<usize>> Const<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

impl Const<u16> {
  pub fn u8(self) -> Const<u8> {
    Const(self.0 as u8)
  }

  pub fn is_u8(&self) -> bool {
    self.0 <= u8::MAX as u16
  }
}

impl<T: Display> Display for Const<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "[{}]", self.0)
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Upvalue<T>(pub T);

impl<T: Into<usize>> Upvalue<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

impl<T: Display> Display for Upvalue<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "u{}", self.0)
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset<T>(pub T);

impl<T: Into<usize>> Offset<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

impl<T: Display> Display for Offset<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Count<T>(pub T);

impl<T: Into<usize>> Count<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

impl<T: Display> Display for Count<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.0)
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mvar<T>(pub T);

impl<T: Into<usize>> Mvar<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

impl<T: Display> Display for Mvar<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "m{}", self.0)
  }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Smi<T>(pub T);

impl<T: Into<i32>> Smi<T> {
  pub fn wide(self) -> i32 {
    self.0.into()
  }
}

impl<T: Display> Display for Smi<T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Op {
  pub fn is_fwd_jump(&self) -> bool {
    use Op::*;
    matches!(
      self,
      Jump { .. }
        | JumpConst { .. }
        | JumpIfFalse { .. }
        | JumpIfFalseConst { .. }
        | JumpIfTrue { .. }
        | JumpIfTrueConst { .. }
    )
  }

  pub fn is_bwd_jump(&self) -> bool {
    use Op::*;
    matches!(self, JumpLoop { .. } | JumpLoopConst { .. })
  }
}

#[rustfmt::skip]
pub mod asm;
