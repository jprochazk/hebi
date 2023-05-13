use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::ptr::NonNull;

use super::module::ModuleId;
use super::ptr::Ptr;
use super::{List, Object, String};
use crate::bytecode::{disasm, opcode as op};
use crate::value::constant::Constant;

#[derive(Debug)]
pub struct Function {
  pub descriptor: Ptr<FunctionDescriptor>,
  pub upvalues: Ptr<List>,
  pub module_id: ModuleId,
}

impl Function {
  pub fn new(
    descriptor: Ptr<FunctionDescriptor>,
    upvalues: Ptr<List>,
    module_id: ModuleId,
  ) -> Self {
    Self {
      descriptor,
      upvalues,
      module_id,
    }
  }
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
pub struct Generator {
  pub descriptor: Ptr<FunctionDescriptor>,
  pub upvalues: Ptr<List>,
  pub module: ModuleId,
}

impl Object for Generator {
  fn type_name(&self) -> &'static str {
    "Generator"
  }
}

impl Display for Generator {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<generator `{}`>", self.descriptor.name)
  }
}

pub struct FunctionDescriptor {
  pub name: Ptr<String>,
  pub is_generator: bool,
  pub params: Params,
  pub upvalues: RefCell<Vec<Upvalue>>,
  pub frame_size: usize,
  pub instructions: NonNull<[u8]>,
  pub constants: NonNull<[Constant]>,
  // TODO: spans
}

#[derive(Debug)]
pub enum Upvalue {
  Register(op::Register),
  Upvalue(op::Upvalue),
}

fn vec_to_nonnull_ptr<T>(v: Vec<T>) -> NonNull<[T]> {
  unsafe { NonNull::new_unchecked(Box::into_raw(v.into_boxed_slice())) }
}

impl FunctionDescriptor {
  pub fn new(
    name: Ptr<String>,
    is_generator: bool,
    params: Params,
    upvalues: Vec<Upvalue>,
    frame_size: usize,
    instructions: Vec<u8>,
    constants: Vec<Constant>,
  ) -> Self {
    let instructions = vec_to_nonnull_ptr(instructions);
    let constants = vec_to_nonnull_ptr(constants);
    Self {
      name,
      is_generator,
      params,
      upvalues: RefCell::new(upvalues),
      frame_size,
      instructions,
      constants,
    }
  }
}

impl FunctionDescriptor {
  pub fn disassemble(&self) -> Disassembly {
    self.disassemble_inner(None)
  }

  pub fn disassemble_as_method(&self, class_name: Ptr<String>) -> Disassembly {
    self.disassemble_inner(Some(class_name))
  }

  fn disassemble_inner(&self, class_name: Option<Ptr<String>>) -> Disassembly {
    Disassembly {
      function: self,
      class_name,
    }
  }
}

pub struct Disassembly<'a> {
  function: &'a FunctionDescriptor,
  class_name: Option<Ptr<String>>,
}

impl<'a> Display for Disassembly<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let function = self.function;

    let (bytecode, constants) =
      unsafe { (function.instructions.as_ref(), function.constants.as_ref()) };

    for constant in constants {
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
    }

    let class_name = match &self.class_name {
      Some(class_name) => format!("{class_name}."),
      None => std::string::String::new(),
    };
    writeln!(
      f,
      "function `{class_name}{}` (registers: {}, length: {}, constants: {})",
      function.name,
      function.frame_size,
      bytecode.len(),
      constants.len(),
    )?;
    if !function.upvalues.borrow().is_empty() {
      writeln!(f, ".upvalues")?;
      for (index, upvalue) in function.upvalues.borrow().iter().enumerate() {
        match upvalue {
          Upvalue::Register(r) => writeln!(f, "  {index} <- {r}",)?,
          Upvalue::Upvalue(u) => writeln!(f, "  {index} <- {u}",)?,
        }
      }
    }
    writeln!(f, ".code")?;
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

impl Debug for FunctionDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("FunctionDescriptor")
      .field("name", &self.name)
      .field("params", &self.params)
      .field("upvalues", &self.upvalues)
      .field("frame_size", &self.frame_size)
      .field("instructions", &unsafe { self.instructions.as_ref() }.len())
      .field("constants", &unsafe { self.constants.as_ref() }.len())
      .finish()
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

  pub fn matches(&self, n: usize) -> bool {
    (self.min as usize) <= n && n <= (self.max as usize)
  }
}

impl Default for Params {
  fn default() -> Self {
    Self::empty()
  }
}
