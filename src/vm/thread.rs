#[macro_use]
mod macros;

mod util;

use std::fmt::{Debug, Display};
use std::mem::take;
use std::ptr::NonNull;

use self::util::*;
use super::dispatch;
use super::dispatch::{dispatch, ControlFlow, Handler};
use super::global::Global;
use crate::bytecode::opcode as op;
use crate::ctx::Context;
use crate::error::{Error, Result};
use crate::object::function::Params;
use crate::object::module::ModuleId;
use crate::object::{Function, FunctionDescriptor, List, Object, Ptr, String};
use crate::span::Span;
use crate::value::constant::Constant;
use crate::value::Value;

pub struct Thread {
  cx: Context,
  global: Global,

  call_frames: Vec<Frame>,
  stack: Vec<Value>,
  stack_base: usize,
  acc: Value,
  pc: usize,
}

impl Thread {
  pub fn new(cx: Context, global: Global) -> Self {
    Thread {
      cx,
      global,

      call_frames: Vec::new(),
      stack: Vec::with_capacity(128),
      stack_base: 0,
      acc: Value::none(),
      pc: 0,
    }
  }

  pub fn call(&mut self, f: Value, args: &[Value]) -> Result<Value> {
    let (stack_base, num_args) = push_args!(self, args);

    self.prepare_call(f, stack_base, num_args, self.pc)?;
    self.run()?;
    Ok(take(&mut self.acc))
  }

  fn run(&mut self) -> Result<()> {
    let instructions = self.call_frames.last_mut().unwrap().instructions;
    let pc = self.pc;

    match dispatch(self, instructions, pc) {
      Ok(ControlFlow::Yield(pc)) => {
        self.pc = pc;
        Ok(())
      }
      Ok(ControlFlow::Return) => {
        self.pc = 0;
        Ok(())
      }
      Err(e) => match e {
        dispatch::Error::Handler(e) => Err(e),
        e @ dispatch::Error::IllegalInstruction => panic!("{e}"),
        e @ dispatch::Error::UnexpectedEnd => panic!("{e}"),
      },
    }
  }

  fn prepare_call(
    &mut self,
    f: Value,
    stack_base: usize,
    num_args: usize,
    return_addr: usize,
  ) -> Result<()> {
    let f = match f.try_to_object() {
      Ok(f) => f,
      Err(f) => fail!(self.cx, 0..0, "cannot call value `{f}`"),
    };

    if f.is::<Function>() {
      let f = unsafe { f.cast_unchecked::<Function>() };
      self.prepare_call_function(f, stack_base, num_args, return_addr)?;
    } else {
      fail!(self.cx, 0..0, "cannot call object `{f}`");
    }

    Ok(())
  }

  fn prepare_call_function(
    &mut self,
    f: Ptr<Function>,
    stack_base: usize,
    num_args: usize,
    return_addr: usize,
  ) -> Result<()> {
    check_args(&self.cx, (0..0).into(), &f.descriptor.params, num_args)?;

    // reset pc
    self.pc = 0;

    // prepare stack
    self.stack_base = stack_base;
    let new_len = stack_base + f.descriptor.frame_size;
    self.stack.resize_with(new_len, Value::none);

    // push frame
    self.call_frames.push(Frame {
      instructions: f.descriptor.instructions,
      constants: f.descriptor.constants,
      upvalues: f.upvalues.clone(),
      frame_size: f.descriptor.frame_size,
      num_args,
      return_addr,
    });

    Ok(())
  }
}

impl Display for Thread {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<thread>")
  }
}

impl Debug for Thread {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Thread")
      .field("stack", &self.stack)
      .field("acc", &self.acc)
      .field("pc", &self.pc)
      .finish()
  }
}

impl Object for Thread {
  fn type_name(&self) -> &'static str {
    "Thread"
  }
}

struct Frame {
  instructions: NonNull<[u8]>,
  constants: NonNull<[Constant]>,
  upvalues: Ptr<List>,
  frame_size: usize,
  num_args: usize,
  return_addr: usize,
}

impl Thread {
  fn get_constant(&self, idx: op::Constant) -> Constant {
    clone_from_raw_slice(current_call_frame!(self).constants.as_ptr(), idx.index())
  }

  fn get_register(&self, reg: op::Register) -> Value {
    debug_assert!(
      self.stack_base + reg.index() < self.stack.len(),
      "register out of bounds {reg:?}"
    );
    let value = unsafe { self.stack.get_unchecked(self.stack_base + reg.index()) };
    value.clone()
  }

  fn set_register(&mut self, reg: op::Register, value: Value) {
    debug_assert!(
      self.stack_base + reg.index() < self.stack.len(),
      "register out of bounds {reg:?}"
    );
    let slot = unsafe { self.stack.get_unchecked_mut(self.stack_base + reg.index()) };
    *slot = value;
  }
}

impl Handler for Thread {
  type Error = Error;

  fn op_load(&mut self, reg: op::Register) -> std::result::Result<(), Self::Error> {
    self.acc = self.get_register(reg);
    println!("load {reg} {}", self.acc);

    Ok(())
  }

  fn op_store(&mut self, reg: op::Register) -> std::result::Result<(), Self::Error> {
    let value = take(&mut self.acc);
    self.set_register(reg, value);

    Ok(())
  }

  fn op_load_const(&mut self, idx: op::Constant) -> std::result::Result<(), Self::Error> {
    self.acc = self.get_constant(idx).into_value();

    Ok(())
  }

  fn op_load_upvalue(&mut self, idx: op::Upvalue) -> std::result::Result<(), Self::Error> {
    let call_frame = current_call_frame!(self);
    let upvalues = &call_frame.upvalues;
    debug_assert!(
      idx.index() < upvalues.len(),
      "upvalue index is out of bounds {idx:?}"
    );
    self.acc = unsafe { call_frame.upvalues.get_unchecked(idx.index()) };

    Ok(())
  }

  fn op_store_upvalue(&mut self, idx: op::Upvalue) -> std::result::Result<(), Self::Error> {
    let call_frame = current_call_frame!(self);
    let upvalues = &call_frame.upvalues;
    debug_assert!(
      idx.index() < upvalues.len(),
      "upvalue index is out of bounds {idx:?}"
    );
    let value = take(&mut self.acc);
    unsafe { call_frame.upvalues.set_unchecked(idx.index(), value) };

    Ok(())
  }

  fn op_load_module_var(&mut self, idx: op::ModuleVar) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_store_module_var(&mut self, idx: op::ModuleVar) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_global(&mut self, name: op::Constant) -> std::result::Result<(), Self::Error> {
    let name = self.get_constant(name).into_value();
    debug_assert_object_type!(name, String);
    let name = unsafe { name.to_object_unchecked().cast_unchecked::<String>() };
    let value = match self.global.globals().get(&name) {
      Some(value) => value,
      None => fail!(self.cx, 0..0, "undefined global {name}"),
    };
    self.acc = value;

    Ok(())
  }

  fn op_store_global(&mut self, name: op::Constant) -> std::result::Result<(), Self::Error> {
    let name = self.get_constant(name).into_value();
    debug_assert_object_type!(name, String);
    let name = unsafe { name.to_object_unchecked().cast_unchecked::<String>() };
    let value = take(&mut self.acc);
    self.global.globals().insert(name, value);

    Ok(())
  }

  fn op_load_field(&mut self, name: op::Constant) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_field_opt(&mut self, name: op::Constant) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_store_field(
    &mut self,
    obj: op::Register,
    name: op::Constant,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_index(&mut self, obj: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_index_opt(&mut self, obj: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_store_index(
    &mut self,
    obj: op::Register,
    key: op::Register,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_self(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_super(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_load_none(&mut self) -> std::result::Result<(), Self::Error> {
    self.acc = Value::none();

    Ok(())
  }

  fn op_load_true(&mut self) -> std::result::Result<(), Self::Error> {
    self.acc = Value::bool(true);

    Ok(())
  }

  fn op_load_false(&mut self) -> std::result::Result<(), Self::Error> {
    self.acc = Value::bool(false);

    Ok(())
  }

  fn op_load_smi(&mut self, smi: op::Smi) -> std::result::Result<(), Self::Error> {
    self.acc = Value::int(smi.value());

    Ok(())
  }

  fn op_make_fn(&mut self, desc: op::Constant) -> std::result::Result<(), Self::Error> {
    let desc = self.get_constant(desc).into_value();
    debug_assert_object_type!(desc, FunctionDescriptor);
    let desc = unsafe {
      desc
        .to_object_unchecked()
        .cast_unchecked::<FunctionDescriptor>()
    };

    // fetch upvalues
    let num_upvalues = desc.upvalues.borrow().len();
    let mut upvalues = Vec::with_capacity(num_upvalues);
    upvalues.resize_with(num_upvalues, Value::none);
    for (i, upvalue) in desc.upvalues.borrow().iter().enumerate() {
      let value = match upvalue {
        crate::object::function::Upvalue::Register(register) => self.get_register(*register),
        crate::object::function::Upvalue::Upvalue(upvalue) => {
          let parent_upvalues = &current_call_frame!(self).upvalues;
          debug_assert!(upvalue.index() < parent_upvalues.len());
          unsafe { parent_upvalues.get_unchecked(upvalue.index()) }
        }
      };
      let slot = unsafe { upvalues.get_unchecked_mut(i) };
      *slot = value;
    }
    let upvalues = self.cx.alloc(List::from(upvalues));

    // TODO: actual module id
    let module_id = ModuleId::null();

    let f = self
      .cx
      .alloc(Function::new(&self.cx, desc, upvalues, module_id));

    self.acc = Value::object(f);

    Ok(())
  }

  fn op_make_class_empty(&mut self, desc: op::Constant) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_class_empty_derived(
    &mut self,
    desc: op::Constant,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_class(
    &mut self,
    desc: op::Constant,
    parts: op::Register,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_class_derived(
    &mut self,
    desc: op::Constant,
    parts: op::Register,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_list(
    &mut self,
    start: op::Register,
    count: op::Count,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_list_empty(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_table(
    &mut self,
    start: op::Register,
    count: op::Count,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_make_table_empty(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_jump(&mut self, offset: op::Offset) -> std::result::Result<op::Offset, Self::Error> {
    Ok(offset)
  }

  fn op_jump_const(&mut self, idx: op::Constant) -> std::result::Result<op::Offset, Self::Error> {
    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };
    Ok(offset)
  }

  fn op_jump_loop(&mut self, offset: op::Offset) -> std::result::Result<op::Offset, Self::Error> {
    Ok(offset)
  }

  fn op_jump_if_false(
    &mut self,
    offset: op::Offset,
  ) -> std::result::Result<super::dispatch::Jump, Self::Error> {
    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Skip),
      false => Ok(super::dispatch::Jump::Move(offset)),
    }
  }

  fn op_jump_if_false_const(
    &mut self,
    idx: op::Constant,
  ) -> std::result::Result<super::dispatch::Jump, Self::Error> {
    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };

    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Move(offset)),
      false => Ok(super::dispatch::Jump::Skip),
    }
  }

  fn op_add(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_sub(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_mul(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_div(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_rem(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_pow(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_inv(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_not(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_eq(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_ne(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_gt(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_ge(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_lt(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_le(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_cmp_type(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_contains(&mut self, lhs: op::Register) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_is_none(&mut self) -> std::result::Result<(), Self::Error> {
    self.acc = Value::bool(self.acc.is_none());
    Ok(())
  }

  fn op_print(&mut self) -> std::result::Result<(), Self::Error> {
    // TODO: allow setting output writer
    println!("{}", self.acc);
    Ok(())
  }

  fn op_print_n(
    &mut self,
    start: op::Register,
    count: op::Count,
  ) -> std::result::Result<(), Self::Error> {
    debug_assert!(self.stack_base + start.index() + count.value() < self.stack.len());

    let start = start.index();
    let end = start + count.value();
    for index in start..end {
      let value = self.get_register(op::Register(index as u32));
      print!("{value}");
    }
    println!();

    Ok(())
  }

  fn op_call(
    &mut self,
    return_addr: usize,
    callee: op::Register,
    args: op::Count,
  ) -> std::result::Result<dispatch::LoadFrame, Self::Error> {
    let f = self.get_register(callee);
    let start = self.stack_base + callee.index() + 1;
    let (stack_base, num_args) = push_args!(self, f.clone(), range(start, start + args.value()));
    self.prepare_call(f, stack_base, num_args, return_addr)?;
    Ok(dispatch::LoadFrame {
      bytecode: current_call_frame!(self).instructions,
      pc: self.pc,
    })
  }

  fn op_call0(
    &mut self,
    return_addr: usize,
  ) -> std::result::Result<dispatch::LoadFrame, Self::Error> {
    let f = take(&mut self.acc);
    let stack_base = self.stack.len();
    self.prepare_call(f, stack_base, 0, return_addr)?;
    Ok(dispatch::LoadFrame {
      bytecode: current_call_frame!(self).instructions,
      pc: self.pc,
    })
  }

  fn op_import(
    &mut self,
    path: op::Constant,
    dst: op::Register,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_return(&mut self) -> std::result::Result<dispatch::Return, Self::Error> {
    // return value is in the accumulator

    // pop frame
    let frame = self.call_frames.pop().unwrap();

    // truncate stack
    self.stack.truncate(self.stack.len() - frame.frame_size);

    Ok(match self.call_frames.last() {
      Some(current_frame) => {
        self.stack_base -= current_frame.frame_size;
        self.pc = frame.return_addr;
        // restore pc
        dispatch::Return::LoadFrame(dispatch::LoadFrame {
          bytecode: current_frame.instructions,
          pc: self.pc,
        })
      }
      None => dispatch::Return::Yield,
    })
  }

  fn op_yield(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }
}
