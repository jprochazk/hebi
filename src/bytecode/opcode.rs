#![allow(non_camel_case_types, non_upper_case_globals)]

#[macro_use]
mod macros;

use super::disasm;
use super::operands::{Operand, Width};

// TODO: update `docs/emit.md` instruction list once this stops changing

instructions! {
  patch_registers, symbolic, decode, Opcode;
  Nop,
  Wide16,
  Wide32,
  Load(reg: Register),
  Store(reg: Register),
  LoadConst(idx: Constant),
  LoadUpvalue(idx: Upvalue),
  StoreUpvalue(idx: Upvalue),
  LoadModuleVar(idx: ModuleVar),
  StoreModuleVar(idx: ModuleVar),
  LoadGlobal(name: Constant),
  StoreGlobal(name: Constant),
  LoadField(name: Constant),
  LoadFieldOpt(name: Constant),
  StoreField(obj: Register, name: Constant),
  LoadIndex(obj: Register),
  LoadIndexOpt(obj: Register),
  StoreIndex(obj: Register, key: Register),
  LoadSelf,
  LoadSuper,
  LoadNone,
  LoadTrue,
  LoadFalse,
  LoadSmi(value: Smi),
  MakeFn(desc: Constant),
  MakeClass(desc: Constant),
  MakeClassDerived(desc: Constant),
  // TODO: MakeListConst / MakeTableConst for statically known values
  MakeList(start: Register, count: Count),
  MakeListEmpty,
  MakeTable(start: Register, count: Count),
  MakeTableEmpty,
  Jump(offset: Offset),
  JumpConst(offset: Constant),
  JumpLoop(offset: Offset),
  JumpIfFalse(offset: Offset),
  JumpIfFalseConst(offset: Constant),
  Add(lhs: Register),
  Sub(lhs: Register),
  Mul(lhs: Register),
  Div(lhs: Register),
  Rem(lhs: Register),
  Pow(lhs: Register),
  Inv,
  Not,
  CmpEq(lhs: Register),
  CmpNe(lhs: Register),
  CmpGt(lhs: Register),
  CmpGe(lhs: Register),
  CmpLt(lhs: Register),
  CmpLe(lhs: Register),
  CmpType(lhs: Register),
  Contains(lhs: Register),
  IsNone,
  Print,
  PrintN(start: Register, count: Count),
  Call(callee: Register, args: Count),
  Call0,
  Import(path: Constant, dst: Register),
  Return,
  Yield,
}

operand_type!(Register, u32, "r{v}");
operand_type!(Constant, u32, "[{v}]");
operand_type!(Upvalue, u32, "^{v}");
operand_type!(ModuleVar, u32, "{v}");
operand_type!(Offset, u32, "{v}");
operand_type!(Smi, i32, "{v}");
operand_type!(Count, u32, "{v}");

impl Constant {
  pub fn index(&self) -> usize {
    self.0 as usize
  }
}

pub trait Operands {
  type Operands: Operand + Sized;
}

pub trait Instruction: disasm::Disassemble + private::Sealed {
  fn opcode(&self) -> Opcode;

  /// Encode the instruction into `buf`.
  ///
  /// This writes the prefix, opcode, and operands.
  ///
  /// Only writes the prefix if the operands overflow.
  fn encode(&self, buf: &mut Vec<u8>);

  fn is_jump(&self) -> bool {
    matches!(
      self.opcode(),
      Opcode::Jump
        | Opcode::JumpConst
        | Opcode::JumpLoop
        | Opcode::JumpIfFalse
        | Opcode::JumpIfFalseConst
    )
  }
}

fn read_instruction(buf: &[u8]) -> Option<(Width, Opcode, &[u8])> {
  let width = Width::decode(buf);
  let (opcode, operands) = match width {
    Width::Normal => (buf[0], &buf[1..]),
    Width::Wide16 | Width::Wide32 => (buf[1], &buf[2..]),
  };
  let opcode = Opcode::try_from(opcode).ok()?;
  Some((width, opcode, operands))
}

fn read_instruction_mut(buf: &mut [u8]) -> Option<(Width, Opcode, &mut [u8])> {
  let width = Width::decode(buf);
  let (opcode, operands) = match width {
    Width::Normal => (buf[0], &mut buf[1..]),
    Width::Wide16 | Width::Wide32 => (buf[1], &mut buf[2..]),
  };
  let opcode = Opcode::try_from(opcode).ok()?;
  Some((width, opcode, operands))
}

mod private {
  pub trait Sealed {}
}

#[cfg(test)]
mod tests;
