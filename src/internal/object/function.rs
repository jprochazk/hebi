use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::ptr::NonNull;

use super::module::ModuleId;
use super::ptr::Ptr;
use super::{Any, List, Object, ReturnAddr, Str};
use crate::internal::bytecode::{disasm, opcode as op};
use crate::internal::error::Result;
use crate::internal::object;
use crate::internal::value::constant::Constant;
use crate::internal::value::Value;
use crate::internal::vm::dispatch::LoadFrame;
use crate::internal::vm::thread::util::check_args;
use crate::internal::vm::thread::{Args, CallResult, Frame, Slot0, Thread};
use crate::public::Scope;

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

  pub fn prepare_call_empty_unchecked(
    this: Ptr<Self>,
    thread: &mut Thread,
    return_addr: ReturnAddr,
  ) {
    let function = this.as_ref();
    let descriptor = function.descriptor.as_ref();
    debug_assert!(descriptor.params.is_empty());

    let frame_size = descriptor.frame_size;
    let stack = unsafe { thread.stack.as_mut() };

    thread.pc = 0;
    stack
      .frames
      .push(Frame::new(function, stack.regs.len(), return_addr));

    stack.regs.reserve(frame_size);
    if !descriptor.params.has_self {
      stack.regs.push(Value::object(this));
      stack
        .regs
        .extend((0..frame_size - 1).map(|_| Value::none()));
    } else {
      stack.regs.extend((0..frame_size).map(|_| Value::none()));
    }
  }

  pub fn prepare_call(
    this: Ptr<Self>,
    thread: &mut Thread,
    args: Args,
    return_addr: ReturnAddr,
  ) -> Result<LoadFrame> {
    let function = this.as_ref();
    let descriptor = function.descriptor.as_ref();
    let bytecode = descriptor.instructions;
    check_args(&descriptor.params, false, args.count)?;

    let frame_size = descriptor.frame_size;
    let stack = unsafe { thread.stack.as_mut() };

    thread.pc = 0;
    stack
      .frames
      .push(Frame::new(function, stack.regs.len(), return_addr));

    stack.regs.reserve(frame_size);

    if !descriptor.params.has_self {
      stack.regs.push(Value::object(this));
      stack
        .regs
        .extend_from_within(args.start..args.start + args.count);
      stack
        .regs
        .extend((0..frame_size - args.count - 1).map(|_| Value::none()));
    } else {
      stack
        .regs
        .extend_from_within(args.start..args.start + args.count);
      stack
        .regs
        .extend((0..frame_size - args.count).map(|_| Value::none()));
    }

    Ok(LoadFrame { bytecode, pc: 0 })
  }
}

impl Object for Function {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Function"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn call(mut scope: Scope<'_>, this: Ptr<Self>, return_addr: ReturnAddr) -> Result<CallResult> {
    Self::prepare_call(this, &mut scope.thread, scope.args, return_addr)
      .map(|_| CallResult::Dispatch)
  }
}

declare_object_type!(Function);

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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Generator"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }
}

declare_object_type!(Generator);

impl Display for Generator {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<generator `{}`>", self.descriptor.name)
  }
}

pub struct FunctionDescriptor {
  pub name: Ptr<Str>,
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
    name: Ptr<Str>,
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

  pub fn disassemble_as_method(&self, class_name: Ptr<Str>) -> Disassembly {
    self.disassemble_inner(Some(class_name))
  }

  fn disassemble_inner(&self, class_name: Option<Ptr<Str>>) -> Disassembly {
    Disassembly {
      function: self,
      class_name,
    }
  }
}

pub struct Disassembly<'a> {
  function: &'a FunctionDescriptor,
  class_name: Option<Ptr<Str>>,
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
  fn type_name(_: Ptr<Self>) -> &'static str {
    "FunctionDescriptor"
  }

  default_instance_of!();
}

declare_object_type!(FunctionDescriptor);

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

  pub fn is_empty(&self) -> bool {
    self.min == 0 && self.max == 0
  }
}

impl Default for Params {
  fn default() -> Self {
    Self::empty()
  }
}

// TODO: store name and type_name
#[derive(Debug)]
pub struct BoundFunction {
  this: Ptr<Any>, // ClassInstance or ClassProxy
  function: Ptr<Function>,
}

impl BoundFunction {
  pub fn new(this: Ptr<Any>, function: Ptr<Function>) -> Self {
    assert!(object::is_class(&this));

    Self { this, function }
  }
}

impl Display for BoundFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<bound fn `{}`>", self.function.descriptor.name)
  }
}

impl Object for BoundFunction {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "BoundFunction"
  }

  fn instance_of(_: Ptr<Self>, _: Value) -> Result<bool> {
    todo!()
  }

  fn call(mut scope: Scope<'_>, this: Ptr<Self>, return_addr: ReturnAddr) -> Result<CallResult> {
    let bound_function = this.as_ref();
    let function = bound_function.function.as_ref();
    let descriptor = function.descriptor.as_ref();
    check_args(&descriptor.params, true, scope.num_args())?;

    scope.thread.pc = 0;
    let stack = unsafe { scope.thread.stack.as_mut() };
    stack
      .frames
      .push(Frame::new(function, stack.regs.len(), return_addr));

    let _ = scope.enter_nested(
      Slot0::Receiver(Value::object(this.this.clone())),
      scope.args,
      Some(descriptor.frame_size),
    );

    Ok(CallResult::Dispatch)
  }
}

declare_object_type!(BoundFunction);
