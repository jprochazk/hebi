use std::fmt::{Debug, Display};
use std::ptr::NonNull;

use super::module::ModuleId;
use super::ptr::Ptr;
use super::{List, Object, String};
use crate::bytecode::disasm;
use crate::value::constant::Constant;

#[derive(Debug)]
pub struct Function {
  pub descriptor: Ptr<FunctionDescriptor>,
  pub upvalues: Ptr<List>,
  pub module: ModuleId,
}

impl Object for Function {
  fn type_name(&self) -> &'static str {
    "Function"
  }
}

impl Display for Function {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<function `{}`>", self.descriptor.name)
  }
}

#[derive(Debug)]
pub struct FunctionDescriptor {
  pub name: Ptr<String>,
  pub params: Params,
  pub num_upvalues: usize,
  pub frame_size: usize,
  pub instructions: NonNull<[u8]>,
  pub constants: NonNull<[Constant]>,
  // TODO: spans
}

fn vec_to_nonnull_ptr<T>(v: Vec<T>) -> NonNull<[T]> {
  unsafe { NonNull::new_unchecked(Box::into_raw(v.into_boxed_slice())) }
}

impl FunctionDescriptor {
  pub fn new(
    name: Ptr<String>,
    params: Params,
    num_upvalues: usize,
    frame_size: usize,
    instructions: Vec<u8>,
    constants: Vec<Constant>,
  ) -> Self {
    let instructions = vec_to_nonnull_ptr(instructions);
    let constants = vec_to_nonnull_ptr(constants);
    Self {
      name,
      params,
      num_upvalues,
      frame_size,
      instructions,
      constants,
    }
  }
}

impl FunctionDescriptor {
  pub fn disassemble(&self) -> Disassembly {
    Disassembly(self)
  }
}

pub struct Disassembly<'a>(&'a FunctionDescriptor);

impl<'a> Display for Disassembly<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let function = self.0;

    let (bytecode, constants) =
      unsafe { (function.instructions.as_ref(), function.constants.as_ref()) };

    for constant in constants {
      if let Constant::Function(function) = constant {
        writeln!(f, "{}\n", function.disassemble())?;
      }
    }

    writeln!(
      f,
      "function `{}` (registers: {}, upvalues: {}, length: {}, constants: {})",
      function.name,
      function.frame_size,
      function.num_upvalues,
      bytecode.len(),
      constants.len(),
    )?;
    writeln!(
      f,
      "{}",
      disasm::Disassembly::new(bytecode, constants, 2, true)
    )
  }
}

impl Drop for FunctionDescriptor {
  fn drop(&mut self) {
    let _ = unsafe { Box::from_raw(self.instructions.as_ptr()) };
    let _ = unsafe { Box::from_raw(self.constants.as_ptr()) };
  }
}

impl Object for FunctionDescriptor {
  fn type_name(&self) -> &'static str {
    "FunctionDescriptor"
  }
}

impl Display for FunctionDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<function `{}` descriptor>", self.name)
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
  pub has_self: bool,
  pub min: u16,
  pub max: u16,
}

impl Params {
  pub fn empty() -> Self {
    Self {
      has_self: false,
      min: 0,
      max: 0,
    }
  }
}

impl Default for Params {
  fn default() -> Self {
    Self::empty()
  }
}
