#[macro_use]
mod macros;

mod util;

use std::fmt::{Debug, Display};
use std::mem::take;
use std::ptr::NonNull;

use indexmap::IndexMap;

use self::util::*;
use super::dispatch;
use super::dispatch::{dispatch, ControlFlow, Handler};
use super::global::Global;
use crate as hebi;
use crate::bytecode::opcode as op;
use crate::object::class::{ClassInstance, ClassMethod, ClassProxy};
use crate::object::function::Params;
use crate::object::module::{ModuleId, ModuleKind};
use crate::object::native::{
  NativeAsyncFunction, NativeClass, NativeClassInstance, NativeFunction,
};
use crate::object::{
  Any, ClassDescriptor, ClassType, Function, FunctionDescriptor, List, Module, Ptr, Str, Table,
  Type,
};
use crate::value::constant::Constant;
use crate::value::Value;
use crate::{emit, object, syntax, Error, LocalBoxFuture, Scope};

pub struct Thread {
  pub(crate) global: Global,
  pub(crate) stack: NonNull<Stack>,
  acc: Value,
  pc: usize,
  async_frame: Option<AsyncFrame>,
  poll: bool,
}

impl Clone for Thread {
  fn clone(&self) -> Self {
    Self {
      global: self.global.clone(),
      stack: self.stack,
      acc: self.acc.clone(),
      pc: self.pc,
      async_frame: None,
      poll: self.poll,
    }
  }
}

pub struct Stack {
  pub(crate) frames: Vec<Frame>,
  pub(crate) regs: Vec<Value>,
  pub(crate) base: usize,
}

impl Stack {
  pub fn new() -> Self {
    Self {
      frames: Vec::with_capacity(8),
      regs: Vec::with_capacity(64),
      base: 0,
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

      async_frame: None,
      poll: false,
    }
  }

  /* pub fn call(&mut self, f: Value, args: &[Value]) -> hebi::Result<Value> {
    let poll = self.poll;
    self.poll = false;

    let args = self.push_args(args);
    if let Err(e) = self.do_call(f, args, None) {
      self.pop_args(args);
      return Err(e);
    };
    if let Err(e) = self.run() {
      self.pop_args(args);
      return Err(e);
    };
    self.pop_args(args);

    self.poll = poll;

    Ok(take(&mut self.acc))
  } */

  pub async fn call(&mut self, f: Value, args: &[Value]) -> hebi::Result<Value> {
    let poll = self.poll;
    self.poll = true;

    let args = self.push_args(args);
    if let Err(e) = self.do_call(f, args, None) {
      self.pop_args(args);
      return Err(e);
    };
    loop {
      if let Err(e) = self.run() {
        self.pop_args(args);
        return Err(e);
      };
      match self.async_frame.take() {
        Some(frame) => match frame.fut.await {
          Ok(value) => {
            self.acc = value;
            self.pop_args(frame.args);
          }
          Err(e) => {
            self.pop_args(args);
            return Err(e);
          }
        },
        None => break,
      }
    }
    self.pop_args(args);

    self.poll = poll;

    Ok(take(&mut self.acc))
  }

  fn run(&mut self) -> hebi::Result<()> {
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

  fn push_args(&mut self, args: &[Value]) -> Args {
    let start = stack!(self).len();
    let count = args.len();
    stack_mut!(self).extend_from_slice(args);
    Args { start, count }
  }

  fn pop_args(&mut self, args: Args) {
    stack_mut!(self).truncate(args.start)
  }

  /// Args are passed through the stack:
  /// - `self.stack[args_start]` is the first arg
  /// - `self.stack[args_start+num_args]` is the last arg
  ///
  /// If the call pushes a call frame, `return_addr` is stored in that call
  /// frame and the `pc` will be restored to it during `op_return`.
  ///
  /// If `return_addr` is `None`, the resulting call frame when popped will
  /// yield to the VM's host.
  fn do_call(
    &mut self,
    value: Value,
    args: Args,
    return_addr: Option<usize>,
  ) -> hebi::Result<dispatch::Call> {
    let object = match value.try_to_any() {
      Ok(f) => f,
      Err(f) => fail!("cannot call value `{f}`"),
    };

    if object.is::<Function>() {
      let function = unsafe { object.cast_unchecked::<Function>() };
      self.call_function(function, args, return_addr)
    } else if object.is::<ClassMethod>() {
      let method = unsafe { object.cast_unchecked::<ClassMethod>() };
      if method.this().is::<NativeClassInstance>() {
        let this = unsafe { method.this().cast_unchecked::<NativeClassInstance>() };
        let function = method.function();
        self.call_native_method(this, function, args)
      } else {
        let this = method.this();
        let function = method.function();
        self.call_method(this, function, args, return_addr)
      }
    } else if object.is::<ClassType>() {
      let class = unsafe { object.cast_unchecked::<ClassType>() };
      self.init_class(class, args)
    } else if object.is::<NativeFunction>() {
      let function = unsafe { object.cast_unchecked::<NativeFunction>() };
      self.call_native_function(function, args)
    } else if object.is::<NativeAsyncFunction>() {
      let function = unsafe { object.cast_unchecked::<NativeAsyncFunction>() };
      self.call_native_async_function(function, args)
    } else if object.is::<NativeClass>() {
      let class = unsafe { object.cast_unchecked::<NativeClass>() };
      self.init_native_class(class, args)
    } else {
      fail!("cannot call object `{object}`")
    }
  }

  fn call_function(
    &mut self,
    function: Ptr<Function>,
    args: Args,
    return_addr: Option<usize>,
  ) -> hebi::Result<dispatch::Call> {
    check_args(&function.descriptor.params, false, args.count)?;

    self.pc = 0;
    unsafe {
      self.stack.as_mut().base = self.stack.as_ref().regs.len();
    }
    // note that this only works because `self` is
    // syntactically guaranteed to only exist in class methods
    // if that guarantee ever disappears, this will break
    if !function.descriptor.params.has_self {
      // this is a regular function, put `function` in slot 0
      // the slot is already reserved by codegen
      stack_mut!(self).push(Value::object(function.clone()));
    }
    // if the above condition is false, the first arg will be the receiver
    stack_mut!(self).extend_from_within(args.start..args.start + args.count);
    let mut remainder = function.descriptor.frame_size - args.count;
    if !function.descriptor.params.has_self {
      remainder -= 1;
    }
    stack_mut!(self).extend((0..remainder).map(|_| Value::none()));

    call_frames_mut!(self).push(Frame {
      instructions: function.descriptor.instructions,
      constants: function.descriptor.constants,
      upvalues: function.upvalues.clone(),
      frame_size: function.descriptor.frame_size,
      return_addr,
      module_id: function.module_id,
    });

    Ok(
      dispatch::LoadFrame {
        bytecode: function.descriptor.instructions,
        pc: 0,
      }
      .into(),
    )
  }

  fn call_method(
    &mut self,
    this: Ptr<Any>,
    function: Ptr<Any>,
    args: Args,
    return_addr: Option<usize>,
  ) -> hebi::Result<dispatch::Call> {
    let function = unsafe { function.cast_unchecked::<Function>() };
    check_args(&function.descriptor.params, true, args.count)?;

    self.pc = 0;
    unsafe {
      self.stack.as_mut().base = self.stack.as_ref().regs.len();
    }
    // receiver is passed implicitly through the `ClassMethod` wrapper
    stack_mut!(self).push(Value::object(this));
    stack_mut!(self).extend_from_within(args.start..args.start + args.count);
    stack_mut!(self)
      .extend((0..function.descriptor.frame_size - args.count - 1).map(|_| Value::none()));

    call_frames_mut!(self).push(Frame {
      instructions: function.descriptor.instructions,
      constants: function.descriptor.constants,
      upvalues: function.upvalues.clone(),
      frame_size: function.descriptor.frame_size,
      return_addr,
      module_id: function.module_id,
    });

    Ok(
      dispatch::LoadFrame {
        bytecode: function.descriptor.instructions,
        pc: 0,
      }
      .into(),
    )
  }

  // TODO: deduplicate
  // - do_native_call
  // - do_native_async_call
  // - also some kind of push_args-like thing

  fn call_native_method(
    &mut self,
    this: Ptr<NativeClassInstance>,
    function: Ptr<Any>,
    args: Args,
  ) -> hebi::Result<dispatch::Call> {
    let start = stack!(self).len();
    let count = args.count + 1;
    stack_mut!(self).push(Value::object(this));
    stack_mut!(self).extend_from_within(args.start..args.start + args.count);
    let args = Args { start, count };

    if let Some(function) = function.clone_cast::<NativeFunction>() {
      match function.call(self.get_scope(args)) {
        Ok(value) => {
          self.acc = value;
          self.pop_args(args);
          Ok(dispatch::Call::Continue)
        }
        Err(e) => {
          self.pop_args(args);
          Err(e)
        }
      }
    } else {
      let function = unsafe { function.cast_unchecked::<NativeAsyncFunction>() };

      let fut = function.call(self.get_scope(args));
      self.async_frame.replace(AsyncFrame { fut, args });

      Ok(dispatch::Call::Yield)
    }
  }

  // TODO: change to not recurse
  fn init_class(&mut self, class: Ptr<ClassType>, args: Args) -> hebi::Result<dispatch::Call> {
    let instance = self
      .global
      .alloc(ClassInstance::new(self.global.clone(), &class));

    if let Some(init) = class.init.as_ref() {
      check_args(&init.descriptor.params, true, args.count)?;

      let _ = self.call_method(
        instance.clone().into_any(),
        init.clone().into_any(),
        args,
        None,
      )?;
      self.run()?;
    } else if args.count > 0 {
      fail!("expected at most 0 args");
    }

    instance.is_frozen.set(true);

    self.acc = Value::object(instance);

    Ok(dispatch::Call::Continue)
  }

  fn init_native_class(
    &mut self,
    class: Ptr<NativeClass>,
    args: Args,
  ) -> hebi::Result<dispatch::Call> {
    let Some(init) = class.init.clone() else {
      fail!("native class `{}` has no initializer", class.name);
    };

    self.call_native_function(init, args)
  }

  fn call_native_function(
    &mut self,
    function: Ptr<NativeFunction>,
    args: Args,
  ) -> hebi::Result<dispatch::Call> {
    // TODO: put this in a function
    let start = stack!(self).len();
    let count = args.count;
    stack_mut!(self).extend_from_within(args.start..args.start + args.count);
    let args = Args { start, count };
    match function.call(self.get_scope(args)) {
      Ok(value) => {
        self.acc = value;
        self.pop_args(args);
        Ok(dispatch::Call::Continue)
      }
      Err(e) => {
        self.pop_args(args);
        Err(e)
      }
    }
  }

  fn call_native_async_function(
    &mut self,
    function: Ptr<NativeAsyncFunction>,
    args: Args,
  ) -> hebi::Result<dispatch::Call> {
    if !self.poll {
      fail!(
        "cannot call async function `{}` in a non-async context",
        function.name
      );
    }
    let start = stack!(self).len();
    let count = args.count;
    stack_mut!(self).extend_from_within(args.start..args.start + args.count);
    let args = Args { start, count };
    let fut = function.call(self.get_scope(args));
    self.async_frame.replace(AsyncFrame { fut, args });

    Ok(dispatch::Call::Yield)
  }

  fn call_native_field_getter(
    &mut self,
    instance: Ptr<NativeClassInstance>,
    getter: Ptr<NativeFunction>,
  ) -> hebi::Result<()> {
    let start = stack!(self).len();
    let count = 1;
    stack_mut!(self).push(Value::object(instance));
    let args = Args { start, count };
    match getter.call(self.get_scope(args)) {
      Ok(value) => {
        self.acc = value;
        self.pop_args(args);
        Ok(())
      }
      Err(e) => {
        self.pop_args(args);
        Err(e)
      }
    }
  }

  fn call_native_field_setter(
    &mut self,
    instance: Ptr<NativeClassInstance>,
    setter: Ptr<NativeFunction>,
    value: Value,
  ) -> hebi::Result<()> {
    let start = stack!(self).len();
    let count = 2;
    stack_mut!(self).push(Value::object(instance));
    stack_mut!(self).push(value);
    let args = Args { start, count };
    match setter.call(self.get_scope(args)) {
      Ok(value) => {
        self.acc = value;
        self.pop_args(args);
        Ok(())
      }
      Err(e) => {
        self.pop_args(args);
        Err(e)
      }
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
    let mut init = None;
    let fields = fields.unwrap_or_else(|| self.global.alloc(Table::new()));
    let mut methods = IndexMap::with_capacity(desc.methods.len());
    for (key, desc) in desc.methods.iter() {
      let method = self.make_fn(desc.clone());
      if key == &"init" {
        init = Some(method.clone());
      }
      methods.insert(key.clone(), method);
    }
    self.global.alloc(ClassType::new(
      desc.name.clone(),
      init,
      fields,
      methods,
      parent,
    ))
  }

  fn load_module(&mut self, path: Ptr<Str>, return_addr: usize) -> hebi::Result<dispatch::Call> {
    if let Some((module_id, module)) = self.global.get_module_by_name(path.as_str()) {
      // module is in cache
      if self.global.is_module_visited(module_id) {
        fail!("attempted to import partially initialized module {path}");
      }
      self.acc = Value::object(module);
      return Ok(dispatch::Call::Continue);
    }

    // module is not in cache, actually load it
    let module_id = self.global.next_module_id();
    // TODO: native modules
    let module = self.global.load_module(path.as_str())?.to_string();
    let module = syntax::parse(self.global.clone(), &module).map_err(Error::Syntax)?;
    let module = emit::emit(self.global.clone(), &module, path.as_str(), false);
    // println!("{}", module.root.disassemble());
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

    self.do_call(
      Value::object(root.clone()),
      Args::empty(),
      Some(return_addr),
    )
  }

  fn get_empty_scope(&self) -> Scope {
    self.get_scope(Args::empty())
  }

  fn get_scope(&self, args: Args) -> Scope {
    Scope::new(self, args)
  }
}

#[derive(Clone, Copy)]
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
      .field("stack", &self.stack)
      .field("acc", &self.acc)
      .field("pc", &self.pc)
      .finish()
  }
}

pub(crate) struct AsyncFrame {
  fut: LocalBoxFuture<'static, hebi::Result<Value>>,
  args: Args,
}

pub(crate) struct Frame {
  instructions: NonNull<[u8]>,
  constants: NonNull<[Constant]>,
  upvalues: Ptr<List>,
  frame_size: usize,
  return_addr: Option<usize>,
  module_id: ModuleId,
}

impl Thread {
  fn get_constant(&self, idx: op::Constant) -> Constant {
    clone_from_raw_slice(current_call_frame!(self).constants.as_ptr(), idx.index())
  }

  fn get_constant_object<T: Type>(&self, idx: op::Constant) -> Ptr<T> {
    let object = self.get_constant(idx).into_value();
    unsafe { object.to_any_unchecked().cast_unchecked::<T>() }
  }

  // TODO: get_register_as
  fn get_register(&self, reg: op::Register) -> Value {
    debug_assert!(
      stack_base!(self) + reg.index() < stack!(self).len(),
      "register out of bounds {reg:?}"
    );
    unsafe {
      stack!(self)
        .get_unchecked(stack_base!(self) + reg.index())
        .clone()
    }
  }

  fn set_register(&mut self, reg: op::Register, value: Value) {
    debug_assert!(
      stack_base!(self) + reg.index() < stack!(self).len(),
      "register out of bounds {reg:?}"
    );
    unsafe {
      let slot = stack_mut!(self).get_unchecked_mut(stack_base!(self) + reg.index());
      *slot = value;
    };
  }
}

impl Handler for Thread {
  type Error = crate::vm::Error;

  fn op_load(&mut self, reg: op::Register) -> hebi::Result<()> {
    self.acc = self.get_register(reg);

    Ok(())
  }

  fn op_store(&mut self, reg: op::Register) -> hebi::Result<()> {
    let value = take(&mut self.acc);
    self.set_register(reg, value);

    Ok(())
  }

  fn op_load_const(&mut self, idx: op::Constant) -> hebi::Result<()> {
    self.acc = self.get_constant(idx).into_value();

    Ok(())
  }

  fn op_load_upvalue(&mut self, idx: op::Upvalue) -> hebi::Result<()> {
    let call_frame = current_call_frame!(self);
    let upvalues = &call_frame.upvalues;
    debug_assert!(
      idx.index() < upvalues.len(),
      "upvalue index is out of bounds {idx:?}"
    );
    self.acc = unsafe { call_frame.upvalues.get_unchecked(idx.index()) };

    Ok(())
  }

  fn op_store_upvalue(&mut self, idx: op::Upvalue) -> hebi::Result<()> {
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

  fn op_load_module_var(&mut self, idx: op::ModuleVar) -> hebi::Result<()> {
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

  fn op_store_module_var(&mut self, idx: op::ModuleVar) -> hebi::Result<()> {
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

  fn op_load_global(&mut self, name: op::Constant) -> hebi::Result<()> {
    let name = self.get_constant_object::<Str>(name);
    let value = match self.global.get(&name) {
      Some(value) => value,
      None => fail!("undefined global {name}"),
    };
    self.acc = value;

    Ok(())
  }

  fn op_store_global(&mut self, name: op::Constant) -> hebi::Result<()> {
    let name = self.get_constant_object::<Str>(name);
    let value = take(&mut self.acc);
    self.global.set(name, value);

    Ok(())
  }

  fn op_load_field(&mut self, name: op::Constant) -> hebi::Result<()> {
    let name = self.get_constant_object::<Str>(name);
    let receiver = take(&mut self.acc);

    if let Some(instance) = receiver.clone().to_object::<NativeClassInstance>() {
      if let Some(field) = instance.class.fields.get(name.as_str()) {
        // call sets `acc`
        return self.call_native_field_getter(instance.clone(), field.get.clone());
      } else if let Some(method) = instance.class.methods.get(name.as_str()) {
        self.acc = Value::object(self.global.alloc(ClassMethod::new(
          instance.clone().into_any(),
          method.to_object(),
        )));
        return Ok(());
      } else {
        fail!("failed to get field `{name}` on value `{instance}`")
      }
    }

    let value = if let Some(object) = receiver.clone().to_any() {
      match object.named_field(self.get_empty_scope(), name.clone())? {
        Some(value) => value,
        None => fail!("failed to get field `{name}` on value `{object}`"),
      }
    } else {
      // TODO: fields on primitives
      todo!()
    };

    if let (Some(object), Some(value)) = (receiver.to_any(), value.clone().to_any()) {
      if object::is_class(&object) && object::is_callable(&value) {
        self.acc = Value::object(self.global.alloc(ClassMethod::new(object, value)));
        return Ok(());
      }
    }

    self.acc = value;

    Ok(())
  }

  fn op_load_field_opt(&mut self, name: op::Constant) -> hebi::Result<()> {
    let name = self.get_constant_object::<Str>(name);
    let receiver = take(&mut self.acc);

    if receiver.is_none() {
      self.acc = Value::none();
      return Ok(());
    }

    if let Some(instance) = receiver.clone().to_object::<NativeClassInstance>() {
      if let Some(getter) = instance
        .class
        .fields
        .get(name.as_str())
        .map(|f| f.get.clone())
      {
        // call sets `acc`
        return self.call_native_field_getter(instance.clone(), getter);
      } else if let Some(method) = instance.class.methods.get(name.as_str()) {
        self.acc = Value::object(self.global.alloc(ClassMethod::new(
          instance.clone().into_any(),
          method.to_object(),
        )));
        return Ok(());
      } else {
        self.acc = Value::none();
        return Ok(());
      }
    }

    let value = if let Some(object) = receiver.clone().to_any() {
      match object.named_field(self.get_empty_scope(), name)? {
        Some(value) => value,
        None => Value::none(),
      }
    } else {
      // TODO: fields on primitives
      todo!()
    };

    if let (Some(object), Some(value)) = (receiver.to_any(), value.clone().to_any()) {
      if object::is_class(&object) && object::is_callable(&value) {
        self.acc = Value::object(self.global.alloc(ClassMethod::new(object, value)));
        return Ok(());
      }
    }

    self.acc = value;

    Ok(())
  }

  fn op_store_field(&mut self, obj: op::Register, name: op::Constant) -> hebi::Result<()> {
    let name = self.get_constant_object::<Str>(name);
    let receiver = self.get_register(obj);
    let value = take(&mut self.acc);

    if let Some(instance) = receiver.clone().to_object::<NativeClassInstance>() {
      if let Some(setter) = instance
        .class
        .fields
        .get(name.as_str())
        .and_then(|f| f.set.clone())
      {
        return self.call_native_field_setter(instance.clone(), setter, value);
      } else {
        fail!("cannot set field `{name}` on value `{instance}`");
      }
    }

    if let Some(object) = receiver.to_any() {
      object.set_named_field(self.get_empty_scope(), name, value)?;
    } else {
      // TODO: fields on primitives
      todo!()
    }

    Ok(())
  }

  fn op_load_index(&mut self, obj: op::Register) -> hebi::Result<()> {
    let object = self.get_register(obj);
    let key = take(&mut self.acc);

    let value = if let Some(object) = object.to_any() {
      match object.keyed_field(self.get_empty_scope(), key.clone())? {
        Some(value) => value,
        None => fail!("failed to get field `{key}` on value `{object}`"),
      }
    } else {
      // TODO: fields on primitives
      todo!()
    };

    self.acc = value;

    Ok(())
  }

  fn op_load_index_opt(&mut self, obj: op::Register) -> hebi::Result<()> {
    let object = self.get_register(obj);
    let key = take(&mut self.acc);

    if object.is_none() {
      self.acc = Value::none();
      return Ok(());
    }

    let value = if let Some(object) = object.to_any() {
      match object.keyed_field(self.get_empty_scope(), key)? {
        Some(value) => value,
        None => Value::none(),
      }
    } else {
      // TODO: fields on primitives
      todo!()
    };

    self.acc = value;

    Ok(())
  }

  fn op_store_index(&mut self, obj: op::Register, key: op::Register) -> hebi::Result<()> {
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

  fn op_load_self(&mut self) -> hebi::Result<()> {
    let this = self.get_register(op::Register(0));

    let this = match this.try_to_object::<ClassProxy>() {
      Ok(proxy) => Value::object(proxy.this.clone()),
      Err(value) => value,
    };

    self.acc = this;
    Ok(())
  }

  fn op_load_super(&mut self) -> hebi::Result<()> {
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

  fn op_load_none(&mut self) -> hebi::Result<()> {
    self.acc = Value::none();

    Ok(())
  }

  fn op_load_true(&mut self) -> hebi::Result<()> {
    self.acc = Value::bool(true);

    Ok(())
  }

  fn op_load_false(&mut self) -> hebi::Result<()> {
    self.acc = Value::bool(false);

    Ok(())
  }

  fn op_load_smi(&mut self, smi: op::Smi) -> hebi::Result<()> {
    self.acc = Value::int(smi.value());

    Ok(())
  }

  fn op_make_fn(&mut self, desc: op::Constant) -> hebi::Result<()> {
    let desc = self.get_constant_object::<FunctionDescriptor>(desc);

    // fetch upvalues
    let f = self.make_fn(desc);

    self.acc = Value::object(f);

    Ok(())
  }

  fn op_make_class(&mut self, desc: op::Constant) -> hebi::Result<()> {
    let desc = self.get_constant_object::<ClassDescriptor>(desc);

    let class = self.make_class(desc, None, None);

    self.acc = Value::object(class);

    Ok(())
  }

  fn op_make_class_derived(&mut self, desc: op::Constant) -> hebi::Result<()> {
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

  fn op_make_data_class(&mut self, desc: op::Constant, parts: op::Register) -> hebi::Result<()> {
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

  fn op_make_data_class_derived(
    &mut self,
    desc: op::Constant,
    parts: op::Register,
  ) -> hebi::Result<()> {
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

  fn op_finalize_class(&mut self) -> Result<(), Self::Error> {
    todo!()
  }

  fn op_make_list(&mut self, start: op::Register, count: op::Count) -> hebi::Result<()> {
    let list = List::with_capacity(count.value());
    for reg in start.iter(count, 1) {
      list.push(self.get_register(reg));
    }
    self.acc = Value::object(self.global.alloc(list));
    Ok(())
  }

  fn op_make_list_empty(&mut self) -> hebi::Result<()> {
    self.acc = Value::object(self.global.alloc(List::new()));
    Ok(())
  }

  fn op_make_table(&mut self, start: op::Register, count: op::Count) -> hebi::Result<()> {
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

  fn op_make_table_empty(&mut self) -> hebi::Result<()> {
    self.acc = Value::object(self.global.alloc(Table::new()));
    Ok(())
  }

  fn op_jump(&mut self, offset: op::Offset) -> hebi::Result<op::Offset> {
    Ok(offset)
  }

  fn op_jump_const(&mut self, idx: op::Constant) -> hebi::Result<op::Offset> {
    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };
    Ok(offset)
  }

  fn op_jump_loop(&mut self, offset: op::Offset) -> hebi::Result<op::Offset> {
    Ok(offset)
  }

  fn op_jump_if_false(&mut self, offset: op::Offset) -> hebi::Result<super::dispatch::Jump> {
    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Skip),
      false => Ok(super::dispatch::Jump::Move(offset)),
    }
  }

  fn op_jump_if_false_const(&mut self, idx: op::Constant) -> hebi::Result<super::dispatch::Jump> {
    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };

    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Move(offset)),
      false => Ok(super::dispatch::Jump::Skip),
    }
  }

  fn op_add(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::int(lhs + rhs),
      f64 => Value::float(lhs + rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_sub(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::int(lhs - rhs),
      f64 => Value::float(lhs - rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_mul(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::int(lhs * rhs),
      f64 => Value::float(lhs * rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_div(&mut self, lhs: op::Register) -> hebi::Result<()> {
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
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_rem(&mut self, lhs: op::Register) -> hebi::Result<()> {
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
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_pow(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::float((lhs as f64).powf(rhs as f64)),
      f64 => Value::float(lhs.powf(rhs)),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_inv(&mut self) -> hebi::Result<()> {
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
      let _ = unsafe { value.to_any_unchecked() };
      todo!()
    } else {
      unreachable!()
    };
    self.acc = value;
    Ok(())
  }

  fn op_not(&mut self) -> hebi::Result<()> {
    let value = take(&mut self.acc);
    let value = Value::bool(!is_truthy(value));
    self.acc = value;
    Ok(())
  }

  fn op_cmp_eq(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs == rhs),
      f64 => Value::bool(lhs == rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_ne(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs != rhs),
      f64 => Value::bool(lhs != rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_gt(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs > rhs),
      f64 => Value::bool(lhs > rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_ge(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs >= rhs),
      f64 => Value::bool(lhs >= rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_lt(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs < rhs),
      f64 => Value::bool(lhs < rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_le(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let value = binary!(lhs, rhs {
      i32 => Value::bool(lhs <= rhs),
      f64 => Value::bool(lhs <= rhs),
      any => todo!(),
    });
    self.acc = value;
    Ok(())
  }

  fn op_cmp_type(&mut self, lhs: op::Register) -> hebi::Result<()> {
    let lhs = self.get_register(lhs);
    let rhs = take(&mut self.acc);
    let same_type = (lhs.is_int() && rhs.is_int())
      || (lhs.is_float() && rhs.is_float())
      || (lhs.is_bool() && rhs.is_bool())
      || (lhs.is_none() && rhs.is_none());
    if !same_type {
      // compare types of objects
      todo!()
    }
    self.acc = Value::bool(same_type);
    Ok(())
  }

  fn op_contains(&mut self, lhs: op::Register) -> hebi::Result<()> {
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

  fn op_is_none(&mut self) -> hebi::Result<()> {
    self.acc = Value::bool(self.acc.is_none());
    Ok(())
  }

  fn op_print(&mut self) -> hebi::Result<()> {
    // TODO: allow setting output writer
    println!("{}", take(&mut self.acc));
    Ok(())
  }

  fn op_print_n(&mut self, start: op::Register, count: op::Count) -> hebi::Result<()> {
    debug_assert!(stack_base!(self) + start.index() + count.value() <= stack!(self).len());

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
  ) -> hebi::Result<dispatch::Call> {
    let f = self.get_register(callee);
    let args = Args {
      start: stack_base!(self) + callee.index() + 1,
      count: args.value(),
    };
    self.do_call(f, args, Some(return_addr))
  }

  fn op_call0(&mut self, return_addr: usize) -> hebi::Result<dispatch::Call> {
    let f = take(&mut self.acc);
    let args = Args {
      start: stack!(self).len(),
      count: 0,
    };
    self.do_call(f, args, Some(return_addr))
  }

  fn op_import(&mut self, path: op::Constant, return_addr: usize) -> hebi::Result<dispatch::Call> {
    let path = self.get_constant_object::<Str>(path);
    self.load_module(path, return_addr)
  }

  fn op_finalize_module(&mut self) -> Result<(), Self::Error> {
    let module_id = current_call_frame!(self).module_id;
    self.global.finish_module(module_id, true);

    let module = unsafe { self.global.get_module_by_id(module_id).unwrap_unchecked() };
    self.acc = Value::object(module);

    Ok(())
  }

  fn op_return(&mut self) -> hebi::Result<dispatch::Return> {
    // return value is in the accumulator

    // pop frame
    let frame = call_frames_mut!(self).pop().unwrap();

    // truncate stack
    let truncate_to = stack!(self).len() - frame.frame_size;
    stack_mut!(self).truncate(truncate_to);

    Ok(match call_frames!(self).last() {
      Some(current_frame) => {
        *stack_base_mut!(self) -= current_frame.frame_size;
        if let Some(return_addr) = frame.return_addr {
          // restore pc
          self.pc = return_addr;
          dispatch::Return::LoadFrame(dispatch::LoadFrame {
            bytecode: current_frame.instructions,
            pc: self.pc,
          })
        } else {
          dispatch::Return::Yield
        }
      }
      None => dispatch::Return::Yield,
    })
  }

  fn op_yield(&mut self) -> hebi::Result<()> {
    todo!()
  }
}
