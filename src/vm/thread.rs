#[macro_use]
mod macros;

pub mod util;

use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::mem::take;
use std::ptr::NonNull;

use indexmap::IndexMap;

use self::util::*;
use super::dispatch::{dispatch, Call, ControlFlow, Handler, LoadFrame, Return};
use super::global::Global;
use crate::bytecode::opcode as op;
use crate::error::{Error, Result};
use crate::object::class::{ClassInstance, ClassProxy};
use crate::object::function::Params;
use crate::object::module::{ModuleId, ModuleKind};
use crate::object::native::LocalBoxFuture;
use crate::object::{
  Any, ClassDescriptor, ClassType, Function, FunctionDescriptor, List, Module, Object, Ptr, Str,
  Table, Type,
};
use crate::public::Scope;
use crate::util::JoinIter;
use crate::value::constant::Constant;
use crate::value::Value;
use crate::{codegen, syntax};

pub struct Thread {
  pub(crate) global: Global,
  pub(crate) stack: NonNull<Stack>,
  acc: Value,
  pub(crate) pc: usize,
  poll: Option<AsyncFrame>,
}

impl Clone for Thread {
  fn clone(&self) -> Self {
    Self {
      global: self.global.clone(),
      stack: self.stack,
      acc: self.acc.clone(),
      pc: self.pc,
      poll: None,
    }
  }
}

#[derive(Debug)]
pub struct Stack {
  pub(crate) frames: Vec<Frame>,
  pub(crate) regs: Vec<Value>,
}

impl Stack {
  pub fn new() -> Self {
    Self {
      frames: Vec::with_capacity(8),
      regs: Vec::with_capacity(64),
    }
  }
}

impl Thread {
  pub fn new(global: Global, stack: NonNull<Stack>) -> Self {
    Thread {
      global,

      stack,
      acc: Value::none(),
      pc: 0,

      poll: None,
    }
  }

  fn unwind_stack(&mut self, stop_at_index: Option<usize>) {
    let stack = unsafe { self.stack.as_mut() };
    let start = stop_at_index.map(|v| v + 1).unwrap_or(0);
    for frame in stack.frames.drain(start..).rev() {
      stack.regs.truncate(frame.stack_base);
    }
  }

  pub async fn entry(&mut self, main: Ptr<Function>) -> Result<Value> {
    Function::prepare_call_empty_unchecked(main.clone(), self, None);
    loop {
      match self.run() {
        Ok(()) => {
          if let Some(frame) = self.poll.take() {
            let result = frame.fut.await;
            self.truncate_stack(frame.stack_base);
            match result {
              Ok(value) => {
                self.acc = value;
                continue;
              }
              Err(e) => break Err(e),
            }
          } else {
            let value = take(&mut self.acc);
            if !unsafe { self.stack.as_ref().regs.is_empty() } {
              eprintln!("{self:?}");
              panic!("stack is not empty upon exit from vm.entry");
            }
            break Ok(value);
          }
        }
        Err(e) => {
          self.unwind_stack(None);
          if !unsafe { self.stack.as_ref().regs.is_empty() } {
            eprintln!("{self:?}");
            panic!("stack is not empty upon exit from vm.entry");
          }
          break Err(e);
        }
      }
    }
  }

  pub async fn call(&mut self, callable: Ptr<Any>, args: &[Value]) -> Result<Value> {
    let current_frame_index = unsafe { self.stack.as_ref().frames.len() };

    let args = self.push_args(args);
    let result = match callable.call(self.get_scope(args), None) {
      Ok(call) => match call {
        CallResult::Return(value) => Ok(value),
        CallResult::Poll(frame) => {
          // `args` is strictly below `frame.args`,
          // so we don't have to pop them here
          frame.fut.await
        }
        CallResult::Dispatch => {
          // the call pushed a frame onto the call stack,
          // so all we have to do is enter the interpreter
          loop {
            match self.run() {
              Ok(()) => {
                if let Some(frame) = self.poll.take() {
                  let result = frame.fut.await;
                  self.truncate_stack(frame.stack_base);
                  match result {
                    Ok(value) => {
                      self.acc = value;
                      continue;
                    }
                    Err(e) => break Err(e),
                  };
                } else {
                  break Ok(take(&mut self.acc));
                }
              }
              Err(e) => break Err(e),
            }
          }
        }
      },
      Err(e) => Err(e),
    };

    match result {
      Ok(value) => {
        self.pop_args(args);
        Ok(value)
      }
      Err(e) => {
        self.unwind_stack(Some(current_frame_index));
        Err(e)
      }
    }
  }

  fn run(&mut self) -> Result<()> {
    let instructions = current_call_frame_mut!(self).instructions;
    let pc = self.pc;

    match dispatch(self, instructions, pc)? {
      ControlFlow::Yield(pc) => {
        self.pc = pc;
        Ok(())
      }
      ControlFlow::Return => {
        self.pc = 0;
        Ok(())
      }
    }
  }

  pub(crate) fn push_args(&mut self, args: &[Value]) -> Args {
    let start = stack!(self).len();
    let count = args.len();
    stack_mut!(self).extend_from_slice(args);
    Args { start, count }
  }

  pub(crate) fn pop_args(&mut self, args: Args) {
    stack_mut!(self).truncate(args.start)
  }

  pub(crate) fn truncate_stack(&mut self, to: usize) {
    stack_mut!(self).truncate(to)
  }

  fn do_call(&mut self, function: Ptr<Any>, args: Args, return_addr: usize) -> Result<Call> {
    if function.is::<Function>() {
      let function = unsafe { function.cast_unchecked::<Function>() };
      match Function::prepare_call(function, self, args, Some(return_addr)) {
        Ok(frame) => return Ok(Call::LoadFrame(frame)),
        Err(e) => return Err(e),
      };
    }

    match function.call(self.get_scope(args), Some(return_addr)) {
      Ok(call) => match call {
        CallResult::Return(value) => {
          self.acc = value;
          Ok(Call::Continue)
        }
        CallResult::Poll(frame) => {
          self.poll = Some(frame);
          Ok(Call::Yield)
        }
        CallResult::Dispatch => {
          let bytecode = current_call_frame!(self).instructions;
          let pc = 0;
          Ok(Call::LoadFrame(LoadFrame { bytecode, pc }))
        }
      },
      Err(e) => Err(e),
    }
  }

  fn make_fn(&mut self, desc: Ptr<FunctionDescriptor>) -> Ptr<Function> {
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
    let upvalues = self.global.alloc(List::from(upvalues));

    self.global.alloc(Function::new(
      desc,
      upvalues,
      current_call_frame!(self).module_id,
    ))
  }

  fn make_class(
    &mut self,
    desc: Ptr<ClassDescriptor>,
    fields: Option<Ptr<Table>>,
    parent: Option<Ptr<ClassType>>,
  ) -> Ptr<ClassType> {
    let mut init = desc.init.as_ref().map(|init| self.make_fn(init.clone()));
    let fields = fields.unwrap_or_else(|| self.global.alloc(Table::new()));
    let mut methods = IndexMap::with_capacity(desc.methods.len());

    // inherit `init` and methods
    if let Some(parent) = parent.as_ref() {
      if init.is_none() {
        init = parent.init.clone();
      }

      for (key, method) in parent.methods.iter() {
        methods.insert(key.clone(), method.clone());
      }
    }

    for (key, desc) in desc.methods.iter() {
      methods.insert(key.clone(), self.make_fn(desc.clone()));
    }

    self.global.alloc(ClassType::new(
      desc.name.clone(),
      init,
      fields,
      methods,
      parent,
    ))
  }

  fn load_module(&mut self, path: Ptr<Str>, return_addr: usize) -> Result<Call> {
    if let Some((module_id, module)) = self.global.get_module_by_name(path.as_str()) {
      // module is in cache
      if self.global.is_module_visited(module_id) {
        fail!("attempted to import partially initialized module {path}");
      }
      self.acc = Value::object(module);
      return Ok(Call::Continue);
    }

    // module is not in cache, actually load it
    let module_id = self.global.next_module_id();
    let module = self.global.load_module(path.as_str())?.to_string();
    let module = syntax::parse(self.global.clone(), &module).map_err(Error::Syntax)?;
    let module = codegen::emit(self.global.clone(), &module, path.as_str(), false);
    let main = self.global.alloc(Function::new(
      module.root.clone(),
      self.global.alloc(List::new()),
      module_id,
    ));
    let module = self.global.alloc(Module::script(
      self.global.clone(),
      path.clone(),
      main,
      &module.module_vars,
      module_id,
    ));
    self.global.define_module(module_id, path, module.clone());

    let ModuleKind::Script { root } = &module.kind else {
      fail!("expected module kind to be `script`");
    };

    <Function as Object>::call(self.get_empty_scope(), root.clone(), Some(return_addr))?;
    Ok(Call::LoadFrame(LoadFrame {
      bytecode: root.descriptor.instructions,
      pc: 0,
    }))
  }

  fn get_empty_scope(&self) -> Scope {
    self.get_scope(Args::empty())
  }

  fn get_scope(&self, args: Args) -> Scope {
    Scope::new(self, stack!(self).len(), args)
  }

  pub(crate) fn enter_nested_scope(
    &mut self,
    stack_base: usize,
    slot0: Slot0,
    args: Args,
    frame_size: Option<usize>,
  ) -> Scope<'static> {
    let start = stack!(self).len();
    let count = slot0.is_some() as usize + args.count;

    if let Some(slot0) = slot0.get() {
      stack_mut!(self).push(slot0);
    }

    stack_mut!(self).extend_from_within(args.start..args.start + args.count);

    if let Some(frame_size) = frame_size {
      debug_assert!(frame_size >= count);
      stack_mut!(self).extend((0..frame_size - count).map(|_| Value::none()));
    }

    let args = Args { start, count };

    Scope::new(self, stack_base, args)
  }

  pub(crate) fn leave_scope(&mut self, scope: Scope) {
    stack_mut!(self).truncate(scope.args.start);
  }

  fn stack_base(&self) -> usize {
    current_call_frame!(self).stack_base
  }
}

pub enum CallResult {
  Return(Value),
  Poll(AsyncFrame),
  Dispatch,
}

pub enum Slot0 {
  Receiver(Value),
  Function(Value),
  None,
}

impl Slot0 {
  fn get(&self) -> Option<Value> {
    match self {
      Slot0::Receiver(value) => Some(value.clone()),
      Slot0::Function(value) => Some(value.clone()),
      Slot0::None => None,
    }
  }

  fn is_function(&self) -> bool {
    use Slot0::*;
    matches!(self, Function(_))
  }

  fn is_some(&self) -> bool {
    use Slot0::*;
    matches!(self, Receiver(_) | Function(_))
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Args {
  pub start: usize,
  pub count: usize,
}

impl Args {
  pub fn empty() -> Self {
    Self { start: 0, count: 0 }
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
      .field("global", &self.global)
      .field("stack", &unsafe { self.stack.as_ref() })
      .field("acc", &self.acc)
      .field("pc", &self.pc)
      .field("poll", &self.poll)
      .finish()
  }
}

pub struct AsyncFrame {
  pub fut: LocalBoxFuture<'static, Result<Value>>,
  pub stack_base: usize,
}

impl Debug for AsyncFrame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("AsyncFrame")
      .field("fut", &"<...>")
      .field("stack_base", &self.stack_base)
      .finish()
  }
}

pub(crate) struct Frame {
  instructions: NonNull<[u8]>,
  constants: NonNull<[Constant]>,
  upvalues: Ptr<List>,
  stack_base: usize,
  frame_size: usize,
  return_addr: Option<usize>,
  module_id: ModuleId,
}

impl Debug for Frame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Frame")
      .field("instructions", &unsafe { self.instructions.as_ref() })
      .field("constants", &unsafe { self.constants.as_ref() })
      .field("upvalues", &self.upvalues)
      .field("stack_base", &self.stack_base)
      .field("frame_size", &self.frame_size)
      .field("return_addr", &self.return_addr)
      .field("module_id", &self.module_id)
      .finish()
  }
}

impl Frame {
  pub(crate) fn new(f: &Function, stack_base: usize, return_addr: Option<usize>) -> Self {
    let desc = f.descriptor.as_ref();

    Self {
      instructions: desc.instructions,
      constants: desc.constants,
      upvalues: f.upvalues.clone(),
      stack_base,
      frame_size: desc.frame_size,
      return_addr,
      module_id: f.module_id,
    }
  }
}

impl Thread {
  fn get_constant(&self, idx: op::Constant) -> Constant {
    clone_from_raw_slice(current_call_frame!(self).constants.as_ptr(), idx.index())
  }

  fn get_constant_object<T: Type>(&self, idx: op::Constant) -> Ptr<T> {
    let object = self.get_constant(idx).into_value();
    unsafe { object.to_any_unchecked().cast_unchecked::<T>() }
  }

  fn get_register(&self, reg: op::Register) -> Value {
    debug_assert!(
      self.stack_base() + reg.index() < stack!(self).len(),
      "register out of bounds {reg:?}"
    );
    unsafe {
      stack!(self)
        .get_unchecked(self.stack_base() + reg.index())
        .clone()
    }
  }

  fn set_register(&mut self, reg: op::Register, value: Value) {
    debug_assert!(
      self.stack_base() + reg.index() < stack!(self).len(),
      "register out of bounds {reg:?}"
    );
    unsafe {
      let slot = stack_mut!(self).get_unchecked_mut(self.stack_base() + reg.index());
      *slot = value;
    };
  }

  #[cfg(not(feature = "__disable_verbose_logs"))]
  fn print_stack(&self) {
    let base = current_call_frame!(self).stack_base;
    let stack = &stack!(self)[base..];
    println!("  stack: [{}]", stack.iter().join(", "));
    println!("  acc: {}", self.acc);
  }
  #[cfg(feature = "__disable_verbose_logs")]
  fn print_stack(&self) {}
}

macro_rules! vprintln {
  ($($tt:tt)*) => {{
    #[cfg(not(feature="__disable_verbose_logs"))]
    {
      println!($($tt)*);
    }
  }}
}

impl Handler for Thread {
  type Error = crate::vm::Error;

  fn op_load(&mut self, reg: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("load {reg}");

    let value = self.get_register(reg);
    self.acc = value;

    Ok(())
  }

  fn op_store(&mut self, reg: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("store {reg}");

    let value = take(&mut self.acc);
    self.set_register(reg, value);

    Ok(())
  }

  fn op_load_const(&mut self, idx: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("load_const {idx}");

    let value = self.get_constant(idx).into_value();
    self.acc = value;

    Ok(())
  }

  fn op_load_upvalue(&mut self, idx: op::Upvalue) -> Result<()> {
    self.print_stack();
    vprintln!("load_upvalue {idx}");

    let call_frame = current_call_frame!(self);
    let upvalues = &call_frame.upvalues;
    debug_assert!(
      idx.index() < upvalues.len(),
      "upvalue index is out of bounds {idx:?}"
    );
    let value = unsafe { call_frame.upvalues.get_unchecked(idx.index()) };
    self.acc = value;

    Ok(())
  }

  fn op_store_upvalue(&mut self, idx: op::Upvalue) -> Result<()> {
    self.print_stack();
    vprintln!("store_upvalue {idx}");

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

  fn op_load_module_var(&mut self, idx: op::ModuleVar) -> Result<()> {
    self.print_stack();
    vprintln!("load_module_var {idx}");

    let module_id = current_call_frame!(self).module_id;
    let module = match self.global.get_module_by_id(module_id) {
      Some(module) => module,
      None => {
        fail!("failed to get module {module_id}");
      }
    };

    let value = match module.module_vars.get_index(idx.index()) {
      Some(value) => value,
      None => {
        fail!("failed to get module variable {idx}");
      }
    };

    self.acc = value;

    Ok(())
  }

  fn op_store_module_var(&mut self, idx: op::ModuleVar) -> Result<()> {
    self.print_stack();
    vprintln!("store_module_var {idx}");

    let module_id = current_call_frame!(self).module_id;
    let module = match self.global.get_module_by_id(module_id) {
      Some(module) => module,
      None => {
        fail!("failed to get module {module_id}");
      }
    };

    let value = take(&mut self.acc);

    let success = module.module_vars.set_index(idx.index(), value.clone());
    if !success {
      fail!("failed to set module variable {idx} (value={value})");
    };

    Ok(())
  }

  fn op_load_global(&mut self, name: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("load_global {name}");

    let name = self.get_constant_object::<Str>(name);
    let value = match self.global.get(&name) {
      Some(value) => value,
      None => fail!("undefined global {name}"),
    };
    self.acc = value;

    Ok(())
  }

  fn op_store_global(&mut self, name: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("store_global {name}");

    let name = self.get_constant_object::<Str>(name);
    let value = take(&mut self.acc);
    self.global.set(name, value);

    Ok(())
  }

  fn op_load_field(&mut self, name: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("load_field {name}");

    let name = self.get_constant_object::<Str>(name);
    let receiver = take(&mut self.acc);

    // native class fields
    // native class methods
    // class methods

    if let Some(object) = receiver.to_any() {
      self.acc = object.named_field(self.get_empty_scope(), name)?;
    } else {
      // TODO: fields on primitives
      todo!("fields on primitives")
    }

    Ok(())
  }

  fn op_load_field_opt(&mut self, name: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("load_field_opt {name}");

    let name = self.get_constant_object::<Str>(name);
    let receiver = take(&mut self.acc);

    if receiver.is_none() {
      self.acc = Value::none();
      return Ok(());
    }

    if let Some(object) = receiver.to_any() {
      self.acc = object
        .named_field_opt(self.get_empty_scope(), name)?
        .unwrap_or_else(Value::none);
    } else {
      // TODO: fields on primitives
      todo!("fields on primitives")
    }

    Ok(())
  }

  fn op_store_field(&mut self, obj: op::Register, name: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("store_field {obj}, {name}");

    let name = self.get_constant_object::<Str>(name);
    let receiver = self.get_register(obj);
    let value = take(&mut self.acc);

    if let Some(object) = receiver.to_any() {
      object.set_named_field(self.get_empty_scope(), name, value)?;
    } else {
      // TODO: fields on primitives
      todo!()
    }

    Ok(())
  }

  fn op_load_index(&mut self, obj: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("load_index {obj}");

    let object = self.get_register(obj);
    let key = take(&mut self.acc);

    if let Some(object) = object.to_any() {
      self.acc = object.keyed_field(self.get_empty_scope(), key)?;
    } else {
      // TODO: fields on primitives
      todo!()
    };

    Ok(())
  }

  fn op_load_index_opt(&mut self, obj: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("load_index_opt {obj}");

    let object = self.get_register(obj);
    let key = take(&mut self.acc);

    if object.is_none() {
      self.acc = Value::none();
      return Ok(());
    }

    if let Some(object) = object.to_any() {
      self.acc = object
        .keyed_field_opt(self.get_empty_scope(), key)?
        .unwrap_or_else(Value::none);
    } else {
      // TODO: fields on primitives
      todo!()
    };

    Ok(())
  }

  fn op_store_index(&mut self, obj: op::Register, key: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("store_index {obj}, {key}");

    let object = self.get_register(obj);
    let key = self.get_register(key);
    let value = take(&mut self.acc);

    if let Some(object) = object.to_any() {
      object.set_keyed_field(self.get_empty_scope(), key, value)?;
    } else {
      // TODO: fields on primitives
      todo!()
    }

    Ok(())
  }

  fn op_load_self(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("load_self");

    let this = self.get_register(op::Register(0));

    let this = match this.try_to_object::<ClassProxy>() {
      Ok(proxy) => Value::object(proxy.this.clone()),
      Err(value) => value,
    };

    self.acc = this;
    Ok(())
  }

  fn op_load_super(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("load_super");

    let this = self.get_register(op::Register(0));

    let Some(this) = this.to_any() else {
      fail!("`self` is not a class instance");
    };

    let proxy = if let Some(proxy) = this.clone_cast::<ClassProxy>() {
      ClassProxy {
        this: proxy.this.clone(),
        class: proxy.class.parent.clone().unwrap(),
      }
    } else if let Some(this) = this.clone_cast::<ClassInstance>() {
      ClassProxy {
        this: this.clone(),
        class: this.parent.clone().unwrap(),
      }
    } else {
      fail!("{this} is not a class");
    };

    self.acc = Value::object(self.global.alloc(proxy));

    Ok(())
  }

  fn op_load_none(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("load_none");

    self.acc = Value::none();

    Ok(())
  }

  fn op_load_true(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("load_true");

    self.acc = Value::bool(true);

    Ok(())
  }

  fn op_load_false(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("load_false");

    self.acc = Value::bool(false);

    Ok(())
  }

  fn op_load_smi(&mut self, smi: op::Smi) -> Result<()> {
    self.print_stack();
    vprintln!("load_smi {smi}");

    self.acc = Value::int(smi.value());

    Ok(())
  }

  fn op_make_fn(&mut self, desc: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("make_fn {desc}");

    let desc = self.get_constant_object::<FunctionDescriptor>(desc);

    // fetch upvalues
    let f = self.make_fn(desc);

    self.acc = Value::object(f);

    Ok(())
  }

  fn op_make_class(&mut self, desc: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("make_class {desc}");

    let desc = self.get_constant_object::<ClassDescriptor>(desc);

    let class = self.make_class(desc, None, None);

    self.acc = Value::object(class);

    Ok(())
  }

  fn op_make_class_derived(&mut self, desc: op::Constant) -> Result<()> {
    self.print_stack();
    vprintln!("make_class_derived {desc}");

    let desc = self.get_constant_object::<ClassDescriptor>(desc);
    let parent = take(&mut self.acc);

    let Some(parent) = parent.clone().to_object::<ClassType>() else {
      fail!("{parent} is not a class");
    };
    let fields = self.global.alloc(parent.fields.copy());
    let class = self.make_class(desc, Some(fields), Some(parent));

    self.acc = Value::object(class);

    Ok(())
  }

  fn op_make_data_class(&mut self, desc: op::Constant, parts: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("make_data_class {desc}, {parts}");

    let desc = self.get_constant_object::<ClassDescriptor>(desc);

    let fields = self.global.alloc(Table::with_capacity(desc.fields.len()));
    for (offset, key) in desc.fields.keys().enumerate() {
      let value = self.get_register(parts.offset(offset));
      fields.insert(key, value);
    }
    let class = self.make_class(desc, Some(fields), None);

    self.acc = Value::object(class);

    Ok(())
  }

  fn op_make_data_class_derived(&mut self, desc: op::Constant, parts: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("make_data_class_derived {desc}, {parts}");

    let desc = self.get_constant_object::<ClassDescriptor>(desc);
    let parent = self.get_register(parts);

    let Some(parent) = parent.clone().to_object::<ClassType>() else {
      fail!("{parent} is not a class");
    };

    let fields = self.global.alloc(parent.fields.copy());
    for (offset, key) in desc.fields.keys().enumerate() {
      let value = self.get_register(parts.offset(1 + offset));
      fields.insert(key, value);
    }
    let class = self.make_class(desc, Some(fields), Some(parent));

    self.acc = Value::object(class);

    Ok(())
  }

  fn op_make_list(&mut self, start: op::Register, count: op::Count) -> Result<()> {
    self.print_stack();
    vprintln!("make_list {start}, {count}");

    let list = List::with_capacity(count.value());
    for reg in start.iter(count, 1) {
      list.push(self.get_register(reg));
    }
    self.acc = Value::object(self.global.alloc(list));
    Ok(())
  }

  fn op_make_list_empty(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("make_list_empty");

    self.acc = Value::object(self.global.alloc(List::new()));
    Ok(())
  }

  fn op_make_table(&mut self, start: op::Register, count: op::Count) -> Result<()> {
    self.print_stack();
    vprintln!("make_table {start}, {count}");

    let table = Table::with_capacity(count.value());
    for reg in start.iter(count, 2) {
      let key = self.get_register(reg);
      let value = self.get_register(reg.offset(1));

      let Some(key) = key.clone().to_any().and_then(|v| v.cast::<Str>().ok()) else {
        fail!( "`{key}` is not a string");
      };

      table.insert(key, value);
    }
    self.acc = Value::object(self.global.alloc(table));
    Ok(())
  }

  fn op_make_table_empty(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("make_table_empty");

    self.acc = Value::object(self.global.alloc(Table::new()));
    Ok(())
  }

  fn op_jump(&mut self, offset: op::Offset) -> Result<op::Offset> {
    self.print_stack();
    vprintln!("jump {offset}");

    Ok(offset)
  }

  fn op_jump_const(&mut self, idx: op::Constant) -> Result<op::Offset> {
    self.print_stack();
    vprintln!("jump_const {idx}");

    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };
    Ok(offset)
  }

  fn op_jump_loop(&mut self, offset: op::Offset) -> Result<op::Offset> {
    self.print_stack();
    vprintln!("jump_loop {offset}");

    Ok(offset)
  }

  fn op_jump_if_false(&mut self, offset: op::Offset) -> Result<super::dispatch::Jump> {
    self.print_stack();
    vprintln!("jump_if_false {offset}");

    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Skip),
      false => Ok(super::dispatch::Jump::Move(offset)),
    }
  }

  fn op_jump_if_false_const(&mut self, idx: op::Constant) -> Result<super::dispatch::Jump> {
    self.print_stack();
    vprintln!("jump_if_false_const {idx}");

    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };

    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Move(offset)),
      false => Ok(super::dispatch::Jump::Skip),
    }
  }

  fn op_add(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("add {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::int(lhs + rhs),
      f64 => Value::float(lhs + rhs),
      any => lhs.add(self.get_empty_scope(), rhs)?,
    });
    self.acc = value;
    Ok(())
  }

  fn op_sub(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("sub {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::int(lhs - rhs),
      f64 => Value::float(lhs - rhs),
      any => lhs.subtract(self.get_empty_scope(), rhs)?,
    });
    self.acc = value;
    Ok(())
  }

  fn op_mul(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("mul {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::int(lhs * rhs),
      f64 => Value::float(lhs * rhs),
      any => lhs.multiply(self.get_empty_scope(), rhs)?,
    });
    self.acc = value;
    Ok(())
  }

  fn op_div(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("div {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => {
        if rhs != 0 {
          Value::float(lhs as f64 / rhs as f64)
        } else {
          fail!("cannot divide int by zero")
        }
      },
      f64 => Value::float(lhs / rhs),
      any => lhs.divide(self.get_empty_scope(), rhs)?,
    });
    self.acc = value;
    Ok(())
  }

  fn op_rem(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("rem {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => {
        if rhs != 0 {
          Value::float(lhs as f64 % rhs as f64)
        } else {
          fail!("cannot divide int by zero")
        }
      },
      f64 => Value::float(lhs % rhs),
      any => lhs.remainder(self.get_empty_scope(), rhs)?,
    });
    self.acc = value;
    Ok(())
  }

  fn op_pow(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("pow {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::float((lhs as f64).powf(rhs as f64)),
      f64 => Value::float(lhs.powf(rhs)),
      any => lhs.pow(self.get_empty_scope(), rhs)?,
    });
    self.acc = value;
    Ok(())
  }

  fn op_inv(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("inv");

    let value = take(&mut self.acc);
    let value = if value.is_int() {
      let value = unsafe { value.to_int_unchecked() };
      Value::int(-value)
    } else if value.is_float() {
      let value = unsafe { value.to_float_unchecked() };
      Value::float(-value)
    } else if value.is_bool() {
      fail!("cannot invert `bool`")
    } else if value.is_none() {
      fail!("cannot invert `none`")
    } else if value.is_object() {
      let value = unsafe { value.to_any_unchecked() };
      value.invert(self.get_empty_scope())?
    } else {
      unreachable!()
    };
    self.acc = value;
    Ok(())
  }

  fn op_not(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("not");
    let value = take(&mut self.acc);
    let value = Value::bool(!is_truthy(value));
    self.acc = value;
    Ok(())
  }

  fn op_cmp_eq(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_eq {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs == rhs),
      f64 => Value::bool(lhs == rhs),
      any => Value::bool(matches!(lhs.cmp(self.get_empty_scope(), rhs)?, Ordering::Equal)),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_ne(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_ne {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs != rhs),
      f64 => Value::bool(lhs != rhs),
      any => Value::bool(matches!(lhs.cmp(self.get_empty_scope(), rhs)?, Ordering::Greater | Ordering::Less)),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_gt(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_gt {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs > rhs),
      f64 => Value::bool(lhs > rhs),
      any => Value::bool(matches!(lhs.cmp(self.get_empty_scope(), rhs)?, Ordering::Greater)),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_ge(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_ge {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs >= rhs),
      f64 => Value::bool(lhs >= rhs),
      any => Value::bool(matches!(lhs.cmp(self.get_empty_scope(), rhs)?, Ordering::Greater | Ordering::Equal)),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_lt(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_lt {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs < rhs),
      f64 => Value::bool(lhs < rhs),
      any => Value::bool(matches!(lhs.cmp(self.get_empty_scope(), rhs)?, Ordering::Less)),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_le(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_le {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs <= rhs),
      f64 => Value::bool(lhs <= rhs),
      any => Value::bool(matches!(lhs.cmp(self.get_empty_scope(), rhs)?, Ordering::Less | Ordering::Equal)),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_type(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("cmp_type {lhs}");

    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);

    let is_same_type = if lhs.is_object() && rhs.is_object() {
      let lhs = unsafe { lhs.to_any_unchecked() };

      lhs.instance_of(rhs)?
    } else {
      (lhs.is_int() && rhs.is_int())
        || (lhs.is_float() && rhs.is_float())
        || (lhs.is_bool() && rhs.is_bool())
        || (lhs.is_none() && rhs.is_none())
    };

    self.acc = Value::bool(is_same_type);

    Ok(())
  }

  fn op_contains(&mut self, lhs: op::Register) -> Result<()> {
    self.print_stack();
    vprintln!("contains {lhs}");

    // lhs in rhs
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);

    let Some(rhs) = rhs.clone().to_any() else {
      fail!("`{rhs}` is not an object");
    };

    let result = rhs.contains(self.get_empty_scope(), lhs)?;
    self.acc = Value::bool(result);
    Ok(())
  }

  fn op_is_none(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("is_none");

    self.acc = Value::bool(self.acc.is_none());
    Ok(())
  }

  fn op_print(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("print");

    let mut output = self.global.io().output.borrow_mut();
    writeln!(&mut output, "{}", take(&mut self.acc)).map_err(Error::user)?;
    Ok(())
  }

  fn op_print_n(&mut self, start: op::Register, count: op::Count) -> Result<()> {
    self.print_stack();
    vprintln!("print_n {start}, {count}");

    debug_assert!(self.stack_base() + start.index() + count.value() <= stack!(self).len());

    let mut output = self.global.io().output.borrow_mut();
    let values = stack!(self)[start.index()..start.index() + count.value()].iter();
    writeln!(&mut output, "{}", values.join(" ")).map_err(Error::user)?;

    Ok(())
  }

  fn op_call(&mut self, return_addr: usize, callee: op::Register, args: op::Count) -> Result<Call> {
    self.print_stack();
    vprintln!("call {callee}, {args} (ret={return_addr})");

    let function = self.get_register(callee);
    let args = Args {
      start: self.stack_base() + callee.index() + 1,
      count: args.value(),
    };

    let Some(function) = function.clone().to_any() else {
      fail!("`{function}` is not callable");
    };

    self.do_call(function, args, return_addr)
  }

  fn op_call0(&mut self, return_addr: usize) -> Result<Call> {
    self.print_stack();
    vprintln!("call0 (ret={return_addr})");

    let function = take(&mut self.acc);
    let args = Args {
      start: stack!(self).len(),
      count: 0,
    };

    let Some(function) = function.clone().to_any() else {
      fail!("`{function}` is not callable");
    };

    self.do_call(function, args, return_addr)
  }

  fn op_import(&mut self, path: op::Constant, return_addr: usize) -> Result<Call> {
    self.print_stack();
    vprintln!("import {path} (ret={return_addr})");

    let path = self.get_constant_object::<Str>(path);
    self.load_module(path, return_addr)
  }

  fn op_finalize_module(&mut self) -> Result<(), Self::Error> {
    self.print_stack();
    vprintln!("finalize_module");

    let module_id = current_call_frame!(self).module_id;
    self.global.finish_module(module_id, true);

    let module = unsafe { self.global.get_module_by_id(module_id).unwrap_unchecked() };
    self.acc = Value::object(module);

    Ok(())
  }

  fn op_return(&mut self) -> Result<Return> {
    self.print_stack();
    vprintln!("return");

    // return value is in the accumulator

    let stack = unsafe { self.stack.as_mut() };

    // pop frame
    debug_assert!(!stack.frames.is_empty());
    let frame = unsafe { stack.frames.pop().unwrap_unchecked() };

    // truncate stack
    stack.regs.truncate(frame.stack_base);

    if let Some(current_frame) = stack.frames.last() {
      if let Some(return_addr) = frame.return_addr {
        self.pc = return_addr;
        return Ok(Return::LoadFrame(LoadFrame {
          bytecode: current_frame.instructions,
          pc: self.pc,
        }));
      }
    }

    Ok(Return::Yield)
  }

  fn op_yield(&mut self) -> Result<()> {
    self.print_stack();
    vprintln!("yield");

    todo!()
  }
}
