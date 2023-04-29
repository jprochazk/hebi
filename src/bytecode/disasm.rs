use std::fmt::Display;

use super::opcode::symbolic;
use crate::util::{num_digits, JoinIter};
use crate::value::constant::Constant;

pub struct Instruction<'a> {
  pub name: &'a str,
  pub operands: Vec<&'a dyn Display>,
  pub constant: Option<Constant>,
}

pub trait Disassemble {
  fn disassemble(&self, constants: &[Constant]) -> Instruction<'_>;
}

impl<'a> Display for Instruction<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Self {
      name,
      operands,
      constant,
    } = self;

    write!(f, "{name}")?;
    if !operands.is_empty() {
      write!(f, " {}", operands.iter().join(" "))?;
    }
    if let Some(constant) = constant {
      write!(f, "; {constant}")?;
    }
    Ok(())
  }
}

pub struct Disassembly<'a> {
  bytecode: &'a [u8],
  constants: &'a [Constant],
  padding: usize,
  offsets: bool,
}

impl<'a> Disassembly<'a> {
  pub fn new(bytecode: &'a [u8], constants: &'a [Constant], padding: usize, offsets: bool) -> Self {
    Self {
      bytecode,
      constants,
      padding,
      offsets,
    }
  }
}

impl<'a> Display for Disassembly<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut current_remainder = self.bytecode;
    let mut offset = 0;
    let offset_width = num_digits(self.bytecode.len());
    while !current_remainder.is_empty() {
      let (instruction, remainder) = symbolic::decode(current_remainder).ok_or(std::fmt::Error)?;
      let size = (remainder.as_ptr() as usize) - (current_remainder.as_ptr() as usize);
      current_remainder = remainder;
      if self.offsets {
        writeln!(
          f,
          "{:padding$}{offset: <offset_width$} | {}",
          "",
          instruction.disassemble(self.constants),
          padding = self.padding
        )?;
      } else {
        writeln!(
          f,
          "{:padding$}{}",
          "",
          instruction.disassemble(self.constants),
          padding = self.padding
        )?;
      }
      offset += size;
    }
    Ok(())
  }
}
