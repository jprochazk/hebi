use crate::opcode::ty::Width;
use crate::opcode::*;

// TODO:
// - accumulator usage
// - operand usage instead of name e.g. to print `r0` instead of `reg=0`, `[0]`
//   instead of `slot=0`, etc.

pub fn disassemble(bytecode: &[u8], offset: usize) -> Disassembly {
  fn inner(bytecode: &[u8], offset: usize, width: Width) -> Disassembly {
    macro_rules! d {
      ($name:ident, $bytecode:ident, $offset:ident, $width:ident) => {
        <$name>::disassemble(
          &$bytecode[$offset + 1..$offset + 1 + <$name>::size_of_operands($width)],
          $width,
        )
      };
    }

    let opcode = bytecode[offset];
    match opcode {
      ops::Nop => Nop::disassemble(&[], width),
      ops::Wide => inner(bytecode, offset + 1, Width::Double),
      ops::ExtraWide => inner(bytecode, offset + 1, Width::Quad),
      ops::LoadConst => d!(LoadConst, bytecode, offset, width),
      ops::LoadReg => d!(LoadReg, bytecode, offset, width),
      ops::StoreReg => d!(StoreReg, bytecode, offset, width),
      ops::Jump => d!(Jump, bytecode, offset, width),
      ops::JumpIfFalse => d!(JumpIfFalse, bytecode, offset, width),
      ops::Sub => d!(Sub, bytecode, offset, width),
      ops::Print => d!(Print, bytecode, offset, width),
      ops::PushSmallInt => d!(PushSmallInt, bytecode, offset, width),
      ops::CreateEmptyList => d!(CreateEmptyList, bytecode, offset, width),
      ops::ListPush => d!(ListPush, bytecode, offset, width),
      ops::Ret => d!(Ret, bytecode, offset, width),
      ops::Suspend => d!(Suspend, bytecode, offset, width),
      _ => panic!("malformed bytecode: invalid opcode 0x{opcode:02x}"),
    }
  }

  inner(bytecode, offset, Width::Single)
}

fn align() -> usize {
  NAMES.iter().map(|v| v.len()).max().unwrap_or(0)
}

pub(super) struct Operand {
  pub(super) name: &'static str,
  pub(super) value: Box<dyn std::fmt::Display>,
}

pub struct Disassembly {
  pub(super) name: &'static str,
  pub(super) width: Width,
  pub(super) operands: Vec<Operand>,
  pub(super) size: usize,
}

impl Disassembly {
  pub fn has_prefix(&self) -> bool {
    matches!(self.width, Width::Double | Width::Quad)
  }

  pub fn size(&self) -> usize {
    if self.has_prefix() {
      1 + self.size
    } else {
      self.size
    }
  }
}

impl ::std::fmt::Display for Disassembly {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    // print opcode + prefix
    write!(f, "{}{}", self.width.as_str(), self.name)?;

    // print operands
    write!(
      f,
      "{:w$}",
      "",
      w = align() - self.width.as_str().len() - self.name.len()
    )?;
    for Operand { name, value } in self.operands.iter() {
      write!(f, " {name}={value}")?;
    }
    Ok(())
  }
}
