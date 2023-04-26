#![allow(non_camel_case_types, non_upper_case_globals)]

#[macro_use]
mod macros;

use paste::paste;

use super::operands::{Operand, Width};

operand_type!(Register, u32);
operand_type!(Constant, u32);
operand_type!(Upvalue, u32);
operand_type!(ModuleVar, u32);
operand_type!(Offset, u32);

instructions! {
  Nop,
  Wide16,
  Wide32,
  Load(register: Register),
  Store(register: Register),
  LoadConst(index: Constant),
  LoadUpvalue(index: Upvalue),
  StoreUpvalue(index: Upvalue),
  LoadModuleVar(index: ModuleVar),
  StoreModuleVar(index: ModuleVar),
  LoadGlobal(name: Constant),
  StoreGlobal(name: Constant),
  LoadField(name: Constant),
  LoadFieldOpt(name: Constant),
  StoreField(name: Constant, object: Register),
  LoadIndex(key: Register),
  LoadIndexOpt(key: Register),
  StoreIndex(key: Register, object: Register),
  LoadSelf,
  LoadSuper,
  LoadNone,
  LoadTrue,
  LoadFalse,
  LoadSmi(value: i32),
  MakeFn(descriptor: Constant),
  UpvalueReg(source: Register, destination: Upvalue),
  UpvalueSlot(source: Upvalue, destination: Upvalue),
  MakeClass(descriptor: Constant),
  Jump(offset: Offset),
  JumpConst(offset: Constant),
  JumpBack(offset: Offset),
  JumpBackConst(offset: Constant),
  JumpIfFalse(offset: Offset),
  JumpIfFalseConst(offset: Constant),
  Add(rhs: Register),
  Sub(rhs: Register),
  Mul(rhs: Register),
  Div(rhs: Register),
  Rem(rhs: Register),
  Pow(rhs: Register),
  Inv,
  Not,
  CmpEq(rhs: Register),
  CmpNe(rhs: Register),
  CmpGt(rhs: Register),
  CmpGe(rhs: Register),
  CmpLt(rhs: Register),
  CmpLe(rhs: Register),
  CmpType(rhs: Register),
  Contains(rhs: Register),
  Print,
  PrintN(start: Register, count: u32),
  Call(function: Register, args: u32),
  Import(path: Constant, destination: Register),
  Ret,
  Suspend,
}

jump_instructions! {
  Jump,
  JumpBack,
  JumpIfFalse,
}

pub trait Operands {
  type Operands: Operand + Sized;
}

pub trait Encode: Operands {
  fn encode(&self, buf: &mut Vec<u8>);
}

pub trait Decode: Operands {
  fn decode(buf: &[u8], width: Width) -> <Self::Operands as Operand>::Decoded;
}

pub trait Instruction: Operands + Encode + Decode {
  const BYTE: u8;
  const NAME: &'static str;

  fn is_jump(&self) -> bool {
    matches!(
      Self::BYTE,
      Jump::BYTE
        | JumpConst::BYTE
        | JumpBack::BYTE
        | JumpBackConst::BYTE
        | JumpIfFalse::BYTE
        | JumpIfFalseConst::BYTE
    )
  }
}

pub trait JumpInstruction: Instruction {
  type Const: JumpInstruction + Sized;

  fn update_offset(&mut self, offset: Offset);
  fn to_const(self) -> Self::Const;
}
