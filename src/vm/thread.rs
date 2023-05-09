use std::fmt::{Debug, Display};
use std::mem::take;
use std::ptr::NonNull;

use super::dispatch::{dispatch, ControlFlow, Handler};
use super::global::Global;
use crate::bytecode::opcode as op;
use crate::ctx::Context;
use crate::error::{Error, Result};
use crate::object::function::Params;
use crate::object::{Function, List, Object, Ptr, String};
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

impl super::Hebi {
  pub fn new_thread(&self) -> Thread {
    Thread::new(self.cx.clone(), self.global.clone())
  }
}

macro_rules! match_type {
  (
    match $binding:ident {
      $($ty:ident => $body:expr,)*

      $(_ => $default:expr,)?
    }
  ) => {{
    $(
      #[allow(unused_variables)]
      let $binding = match $binding.cast::<$ty>() {
        Ok($binding) => $body,
        Err(v) => v,
      };
    )*
    $({ $default })?
  }};
}

fn clone_from_raw_slice<T: Clone>(ptr: *mut [T], index: usize) -> T {
  debug_assert!(
    index < std::ptr::metadata(ptr),
    "index out of bounds {index}"
  );
  let value = unsafe { std::mem::ManuallyDrop::new(std::ptr::read((ptr as *mut T).add(index))) };
  std::mem::ManuallyDrop::into_inner(value.clone())
}

macro_rules! current_call_frame {
  ($self:ident) => {{
    debug_assert!(!$self.call_frames.is_empty(), "call frame stack is empty");
    unsafe { $self.call_frames.last().unwrap_unchecked() }
  }};
}

macro_rules! current_call_frame_mut {
  ($self:ident) => {{
    debug_assert!(!$self.call_frames.is_empty(), "call frame stack is empty");
    unsafe { $self.call_frames.last_mut().unwrap_unchecked() }
  }};
}

fn check_args(cx: &Context, span: Span, params: &Params, n: usize) -> Result<()> {
  if !params.matches(n) {
    fail!(
      cx,
      span,
      "expected {}..{} params, got {}",
      params.min,
      params.max,
      n,
    );
  }
  Ok(())
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
    if !f.is_object() {
      fail!(self.cx, 0..0, "cannot call value `{f}`");
    }
    let f = unsafe { f.to_object_unchecked() };

    if f.is::<Function>() {
      let f = unsafe { f.cast_unchecked::<Function>() };

      check_args(&self.cx, (0..0).into(), &f.descriptor.params, args.len())?;

      // put args on the stack
      let stack_base = self.stack.len();
      self.stack.extend_from_slice(args);

      // save pc
      if let Some(current_frame) = self.call_frames.last_mut() {
        current_frame.pc = self.pc;
      }
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
        num_args: args.len(),
        pc: 0,
      });
    } else {
      fail!(self.cx, 0..0, "cannot call object `{f}`")
    }

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
        super::dispatch::Error::Handler(e) => Err(e),
        e => panic!("{e}"),
      },
    }
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
  pc: usize,
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

macro_rules! debug_assert_object_type {
  ($value:ident, $ty:ty) => {{
    let value = match $value.clone().to_object() {
      Some(value) => value,
      None => panic!("{} is not an object", stringify!($value)),
    };
    if let Err(e) = value.cast::<$ty>() {
      panic!("{e}");
    }
  }};
}

fn is_truthy(value: Value) -> bool {
  if value.is_bool() {
    return unsafe { value.to_bool_unchecked() };
  }

  if value.is_float() {
    let value = unsafe { value.to_float_unchecked() };
    return !value.is_nan() && value != 0.0;
  }

  if value.is_int() {
    let value = unsafe { value.to_int_unchecked() };
    return value != 0;
  }

  if value.is_none() {
    return false;
  }

  true
}

impl Handler for Thread {
  type Error = Error;

  fn op_load(&mut self, reg: op::Register) -> std::result::Result<(), Self::Error> {
    self.acc = self.get_register(reg);

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
    todo!()
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
      true => Ok(super::dispatch::Jump::Move(offset)),
      false => Ok(super::dispatch::Jump::Skip),
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
    callee: op::Register,
    args: op::Count,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_call0(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_import(
    &mut self,
    path: op::Constant,
    dst: op::Register,
  ) -> std::result::Result<(), Self::Error> {
    todo!()
  }

  fn op_return(&mut self) -> std::result::Result<(), Self::Error> {
    // pop frame
    let frame = self.call_frames.pop().unwrap();

    // truncate stack
    self.stack.truncate(self.stack.len() - frame.frame_size);

    // restore pc
    match self.call_frames.last() {
      Some(current_frame) => self.pc = current_frame.pc,
      None => self.pc = 0,
    };

    Ok(())
  }

  fn op_yield(&mut self) -> std::result::Result<(), Self::Error> {
    todo!()
  }
}
