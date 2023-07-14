pub mod emit;

mod ux;
use ux::u24;

/*
codegen notes:
- for constant indices stored as `u8`, the constant can
  first be loaded into a register, and then used, because
  `LoadConst` stores the constant index as `u16`, allowing
  a much greater range
*/

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reg<T: Into<usize>>(pub T);

impl<T: Into<usize>> Reg<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Const<T: Into<usize>>(pub T);

impl<T: Into<usize>> Const<T> {
  pub fn wide(self) -> usize {
    self.0.into()
  }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Smi<T: Into<i32>>(pub T);

impl<T: Into<i32>> Smi<T> {
  pub fn wide(self) -> i32 {
    self.0.into()
  }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Op {
  Nop,
  Move {
    src: Reg<u8>,
    dst: Reg<u8>,
  },
  LoadConst {
    dst: Reg<u8>,
    idx: Const<u16>,
  },
  LoadUpvalue {
    idx: Const<u16>,
    dst: Reg<u8>,
  },
  SetUpvalue {
    idx: Const<u16>,
    src: Reg<u8>,
  },
  LoadMVar {
    idx: Const<u16>,
    dst: Reg<u8>,
  },
  SetMVar {
    idx: Const<u16>,
    src: Reg<u8>,
  },
  LoadGlobal {
    name: Const<u16>,
    dst: Reg<u8>,
  },
  SetGlobal {
    name: Const<u16>,
    reg: Reg<u8>,
  },
  LoadFieldReg {
    obj: Reg<u8>,
    name: Reg<u8>,
    reg: Reg<u8>,
  },
  LoadFieldConst {
    obj: Reg<u8>,
    name: Const<u8>,
    reg: Reg<u8>,
  },
  LoadFieldOptReg {
    obj: Reg<u8>,
    name: Reg<u8>,
    reg: Reg<u8>,
  },
  LoadFieldOptConst {
    obj: Reg<u8>,
    name: Const<u8>,
    reg: Reg<u8>,
  },
  SetField {
    obj: Reg<u8>,
    name: Reg<u8>,
    reg: Reg<u8>,
  },
  LoadIndex {
    obj: Reg<u8>,
    key: Reg<u8>,
    reg: Reg<u8>,
  },
  LoadIndexOpt {
    obj: Reg<u8>,
    key: Reg<u8>,
    reg: Reg<u8>,
  },
  SetIndex {
    obj: Reg<u8>,
    key: Reg<u8>,
    reg: Reg<u8>,
  },
  LoadSuper {
    dst: Reg<u8>,
  },
  LoadNone {
    dst: Reg<u8>,
  },
  LoadTrue {
    dst: Reg<u8>,
  },
  LoadFalse {
    dst: Reg<u8>,
  },
  LoadSmi {
    value: Smi<i16>,
    dst: Reg<u8>,
  },
  MakeFn {
    desc: Const<u16>,
    dst: Reg<u8>,
  },
  MakeClass {
    desc: Const<u16>,
    dst: Reg<u8>,
  },
  MakeClassDerived {
    desc: Const<u16>,
    dst: Reg<u8>,
  },
  MakeList {
    desc: Const<u16>,
    dst: Reg<u8>,
  },
  MakeListSmall {
    start: Reg<u8>,
    count: u8,
    dst: Reg<u8>,
  },
  MakeListEmpty {
    dst: Reg<u8>,
  },
  MakeTable {
    desc: Const<u16>,
    dst: Reg<u8>,
  },
  MakeTableSmall {
    start: Reg<u8>,
    count: u8,
    dst: Reg<u8>,
  },
  MakeTableEmpty {
    dst: Reg<u8>,
  },
  Jump {
    offset: u24,
  },
  JumpConst {
    offset: Const<u24>,
  },
  JumpLoop {
    offset: u24,
  },
  JumpLoopConst {
    offset: Const<u24>,
  },
  JumpIfFalse {
    offset: u24,
  },
  JumpIfFalseConst {
    offset: Const<u24>,
  },
  Add {
    dst: Reg<u8>,
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Sub {
    dst: Reg<u8>,
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Mul {
    dst: Reg<u8>,
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Div {
    dst: Reg<u8>,
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Rem {
    dst: Reg<u8>,
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Pow {
    dst: Reg<u8>,
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Inv {
    val: Reg<u8>,
  },
  Not {
    val: Reg<u8>,
  },
  CmpEq {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  CmpNe {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  CmpGt {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  CmpGe {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  CmpLt {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  CmpLe {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  CmpType {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  Contains {
    lhs: Reg<u8>,
    rhs: Reg<u8>,
  },
  IsNone {
    val: Reg<u8>,
  },
  Call {
    func: Reg<u8>,
    count: u8,
  },
  Call0 {
    func: Reg<u8>,
  },
  Import {
    path: Const<u16>,
    dst: Reg<u8>,
  },
  FinalizeModule,
  Return {
    val: Reg<u8>,
  },
  Yield {
    val: Reg<u8>,
  },
}
