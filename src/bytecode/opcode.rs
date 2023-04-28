#![allow(non_camel_case_types, non_upper_case_globals)]

#[macro_use]
mod macros;

use super::disasm;
use super::operands::{Operand, Width};

// TODO: disassembly
//
// fn disassemble(buf: &[u8]) -> String
//
// ^ the above function will be generated in `instructions!`
//
// it will match on the byte, if it's a prefix, expand the width, and then read
// the instruction match on the opcode again, and dispatch to the disassembly
// impl for the specific instruction that will produce a Disassembly struct
// which will accept `name: &'static str` and `operands: (A, B, C, ...)`, where
// each operand will impl another trait, so that the output can be customized
// (e.g. registers are printed as `r<index>`, constants as `[0]`)
// on top of that, the entire instruction will be converted back to its symbolic
// representation and passed to an `extra` method, which can be used to print
// stuff in the disassembly after after the `;`
//
//   example:
//     import [0], r0 ; <module `kv`>
//     └┬───┘ └┬────┘ └────────────┬┘
//      name   operands            extra

instructions! {
  patch_registers, symbolic, decode, Opcode;
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
  LoadSmi(value: Smi),
  MakeFn(descriptor: Constant),
  UpvalueReg(source: Register, destination: Upvalue),
  UpvalueSlot(source: Upvalue, destination: Upvalue),
  MakeClass(descriptor: Constant),
  Jump(offset: Offset),
  JumpConst(offset: Constant),
  JumpLoop(offset: Offset),
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
  PrintN(start: Register, count: Count),
  Call(function: Register, args: Count),
  Import(path: Constant, destination: Register),
  Ret,
  Suspend,
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
