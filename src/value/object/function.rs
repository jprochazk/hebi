use std::fmt::{Debug, Display};
use std::ptr::NonNull;

use super::module::ModuleId;
use super::ptr::Ptr;
use super::{List, Object, String};
use crate::op::Instruction;
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
    write!(f, "<function {}>", self.descriptor.name)
  }
}

#[derive(Debug)]
pub struct FunctionDescriptor {
  pub name: Ptr<String>,
  pub params: Params,
  pub num_upvalues: u16,
  pub frame_size: u16,
  pub instructions: NonNull<[Instruction]>,
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
    num_upvalues: u16,
    frame_size: u16,
    instructions: Vec<Instruction>,
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
    write!(f, "<function descriptor {}>", self.name)
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
  pub min: u16,
  pub max: u16,
}

impl Params {
  pub fn empty() -> Self {
    Self { min: 0, max: 0 }
  }
}

impl Default for Params {
  fn default() -> Self {
    Self::empty()
  }
}
