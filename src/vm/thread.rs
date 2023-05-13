#[macro_use]
mod macros;

mod util;

use std::fmt::{Debug, Display};
use std::mem::take;
use std::ptr::NonNull;

use self::util::*;
use super::dispatch::{dispatch, ControlFlow, Handler};
use super::global::Global;
use super::{dispatch, HebiResult};
use crate::bytecode::opcode as op;
use crate::ctx::Context;
use crate::object::function::Params;
use crate::object::module::ModuleId;
use crate::object::{
  class, Function, FunctionDescriptor, List, Module, Object, Ptr, String, Table,
};
use crate::span::Span;
use crate::value::constant::Constant;
use crate::value::Value;
use crate::{emit, syntax};

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

  pub fn call(&mut self, f: Value, args: &[Value]) -> HebiResult<Value> {
    let (stack_base, num_args) = push_args!(self, args);
    self.prepare_call(f, stack_base, num_args, None)?;
    self.run()?;
    Ok(take(&mut self.acc))
  }

  fn run(&mut self) -> HebiResult<()> {
    let instructions = self.call_frames.last_mut().unwrap().instructions;
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

  fn prepare_call(
    &mut self,
    f: Value,
    stack_base: usize,
    num_args: usize,
    return_addr: Option<usize>,
  ) -> HebiResult<()> {
    let f = match f.try_to_object() {
      Ok(f) => f,
      Err(f) => fail!(0..0, "cannot call value `{f}`"),
    };

    if f.is::<Function>() {
      let f = unsafe { f.cast_unchecked::<Function>() };
      self.prepare_call_function(f, stack_base, num_args, return_addr)?;
    } else {
      fail!(0..0, "cannot call object `{f}`");
    }

    Ok(())
  }

  fn prepare_call_function(
    &mut self,
    f: Ptr<Function>,
    stack_base: usize,
    num_args: usize,
    return_addr: Option<usize>,
  ) -> HebiResult<()> {
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
      return_addr,
      module_id: f.module_id,
    });

    Ok(())
  }

  fn load_module(&mut self, path: Ptr<String>) -> HebiResult<Ptr<Module>> {
    if let Some((module_id, module)) = self.global.module_registry().get_by_name(path.as_str()) {
      // module is in cache
      if self.global.module_visited_set().contains(&module_id) {
        fail!(
          0..0,
          "attempted to import partially initialized module {path}"
        );
      }
      return Ok(module);
    }

    // module is not in cache, actually load it
    let module_id = self.global.module_registry_mut().next_module_id();
    // TODO: native modules
    let module = self.global.module_loader().load(path.as_str())?.to_string();
    // TODO: handle parse error properly
    // vm::Result { Vm, User, Parse }
    let module = syntax::parse(&self.cx, &module).expect("parse error");
    let module = emit::emit(&self.cx, &module, path.as_str(), false);
    println!("{}", module.root.disassemble());
    let main = self.cx.alloc(Function::new(
      module.root.clone(),
      self.cx.alloc(List::new()),
      module_id,
    ));
    let module = self.cx.alloc(Module::new(
      &self.cx,
      path.clone(),
      main,
      &module.module_vars,
      module_id,
    ));
    self
      .global
      .module_registry_mut()
      .insert(module_id, path, module.clone());
    self.global.module_visited_set_mut().insert(module_id);

    let result = match self.call(Value::object(module.root.clone()), &[]) {
      Ok(_) => Ok(module),
      Err(e) => {
        self.global.module_registry_mut().remove(module_id);
        Err(e)
      }
    };
    self.global.module_visited_set_mut().remove(&module_id);
    result
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
  return_addr: Option<usize>,
  module_id: ModuleId,
}

impl Thread {
  fn get_constant(&self, idx: op::Constant) -> Constant {
    clone_from_raw_slice(current_call_frame!(self).constants.as_ptr(), idx.index())
  }

  fn get_constant_object<T: Object>(&self, idx: op::Constant) -> Ptr<T> {
    let object = self.get_constant(idx).into_value();
    debug_assert_object_type!(object, T);
    unsafe { object.to_object_unchecked().cast_unchecked::<T>() }
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
  type Error = crate::vm::HebiError;

  fn op_load(&mut self, reg: op::Register) -> HebiResult<()> {
    self.acc = self.get_register(reg);
    println!("load {reg} {}", self.acc);

    Ok(())
  }

  fn op_store(&mut self, reg: op::Register) -> HebiResult<()> {
    let value = take(&mut self.acc);
    self.set_register(reg, value);

    Ok(())
  }

  fn op_load_const(&mut self, idx: op::Constant) -> HebiResult<()> {
    self.acc = self.get_constant(idx).into_value();

    Ok(())
  }

  fn op_load_upvalue(&mut self, idx: op::Upvalue) -> HebiResult<()> {
    let call_frame = current_call_frame!(self);
    let upvalues = &call_frame.upvalues;
    debug_assert!(
      idx.index() < upvalues.len(),
      "upvalue index is out of bounds {idx:?}"
    );
    self.acc = unsafe { call_frame.upvalues.get_unchecked(idx.index()) };

    Ok(())
  }

  fn op_store_upvalue(&mut self, idx: op::Upvalue) -> HebiResult<()> {
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

  fn op_load_module_var(&mut self, idx: op::ModuleVar) -> HebiResult<()> {
    let module_id = current_call_frame!(self).module_id;
    let module = match self.global.module_registry().get_by_id(module_id) {
      Some(module) => module,
      None => {
        fail!(0..0, "failed to get module {module_id}");
      }
    };

    let value = match module.module_vars.get_index(idx.index()) {
      Some(value) => value,
      None => {
        fail!(0..0, "failed to get module variable {idx}");
      }
    };

    self.acc = value;

    Ok(())
  }

  fn op_store_module_var(&mut self, idx: op::ModuleVar) -> HebiResult<()> {
    let module_id = current_call_frame!(self).module_id;
    let module = match self.global.module_registry().get_by_id(module_id) {
      Some(module) => module,
      None => {
        fail!(0..0, "failed to get module {module_id}");
      }
    };

    let value = take(&mut self.acc);

    let success = module.module_vars.set_index(idx.index(), value.clone());
    if !success {
      fail!(0..0, "failed to set module variable {idx} (value={value})");
    };

    Ok(())
  }

  fn op_load_global(&mut self, name: op::Constant) -> HebiResult<()> {
    let name = self.get_constant_object::<String>(name);
    let value = match self.global.globals().get(&name) {
      Some(value) => value,
      None => fail!(0..0, "undefined global {name}"),
    };
    self.acc = value;

    Ok(())
  }

  fn op_store_global(&mut self, name: op::Constant) -> HebiResult<()> {
    let name = self.get_constant_object::<String>(name);
    let value = take(&mut self.acc);
    self.global.globals().insert(name, value);

    Ok(())
  }

  fn op_load_field(&mut self, name: op::Constant) -> HebiResult<()> {
    let name = self.get_constant_object::<String>(name);
    let value = take(&mut self.acc);

    let value = if let Some(object) = value.to_object() {
      match object.named_field(&self.cx, name.as_str())? {
        Some(value) => value,
        None => fail!(0..0, "failed to get field `{name}` on value `{object}`"),
      }
    } else {
      // TODO: fields on primitives
      todo!()
    };

    self.acc = value;

    Ok(())
  }

  fn op_load_field_opt(&mut self, name: op::Constant) -> HebiResult<()> {
    let name = self.get_constant_object::<String>(name);
    let value = take(&mut self.acc);

    if value.is_none() {
      self.acc = Value::none();
      return Ok(());
    }

    let value = if let Some(object) = value.to_object() {
      match object.named_field(&self.cx, name.as_str())? {
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

  fn op_store_field(&mut self, obj: op::Register, name: op::Constant) -> HebiResult<()> {
    let name = self.get_constant_object::<String>(name);
    let object = self.get_register(obj);
    let value = take(&mut self.acc);

    if let Some(object) = object.to_object() {
      object.set_named_field(&self.cx, name.as_str(), value)?;
    } else {
      // TODO: fields on primitives
      todo!()
    }

    Ok(())
  }

  fn op_load_index(&mut self, obj: op::Register) -> HebiResult<()> {
    let object = self.get_register(obj);
    let key = take(&mut self.acc);

    let value = if let Some(object) = object.to_object() {
      match object.keyed_field(&self.cx, key.clone())? {
        Some(value) => value,
        None => fail!(0..0, "failed to get field `{key}` on value `{object}`"),
      }
    } else {
      // TODO: fields on primitives
      todo!()
    };

    self.acc = value;

    Ok(())
  }

  fn op_load_index_opt(&mut self, obj: op::Register) -> HebiResult<()> {
    let object = self.get_register(obj);
    let key = take(&mut self.acc);

    if object.is_none() {
      self.acc = Value::none();
      return Ok(());
    }

    let value = if let Some(object) = object.to_object() {
      match object.keyed_field(&self.cx, key)? {
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

  fn op_store_index(&mut self, obj: op::Register, key: op::Register) -> HebiResult<()> {
    let object = self.get_register(obj);
    let key = self.get_register(key);
    let value = take(&mut self.acc);

    if let Some(object) = object.to_object() {
      object.set_keyed_field(&self.cx, key, value)?;
    } else {
      // TODO: fields on primitives
      todo!()
    }

    Ok(())
  }

  fn op_load_self(&mut self) -> HebiResult<()> {
    self.acc = self.get_register(op::Register(0));
    Ok(())
  }

  fn op_load_super(&mut self) -> HebiResult<()> {
    let this = self.get_register(op::Register(0));

    let Some(this) = this.to_object() else {
      fail!( 0..0, "`self` is not a class instance");
    };

    let proxy = if let Some(proxy) = this.clone_cast::<class::Proxy>() {
      class::Proxy {
        this: proxy.this.clone(),
        class: proxy.class.parent.clone().unwrap(),
      }
    } else if let Some(this) = this.clone_cast::<class::Instance>() {
      class::Proxy {
        this: this.clone(),
        class: this.parent.clone().unwrap(),
      }
    } else {
      fail!(0..0, "{this} is not a class");
    };

    self.acc = Value::object(self.cx.alloc(proxy));

    Ok(())
  }

  fn op_load_none(&mut self) -> HebiResult<()> {
    self.acc = Value::none();

    Ok(())
  }

  fn op_load_true(&mut self) -> HebiResult<()> {
    self.acc = Value::bool(true);

    Ok(())
  }

  fn op_load_false(&mut self) -> HebiResult<()> {
    self.acc = Value::bool(false);

    Ok(())
  }

  fn op_load_smi(&mut self, smi: op::Smi) -> HebiResult<()> {
    self.acc = Value::int(smi.value());

    Ok(())
  }

  fn op_make_fn(&mut self, desc: op::Constant) -> HebiResult<()> {
    let desc = self.get_constant_object::<FunctionDescriptor>(desc);

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

    let f = self.cx.alloc(Function::new(
      desc,
      upvalues,
      current_call_frame!(self).module_id,
    ));

    self.acc = Value::object(f);

    Ok(())
  }

  fn op_make_class_empty(&mut self, desc: op::Constant) -> HebiResult<()> {
    todo!()
  }

  fn op_make_class_empty_derived(&mut self, desc: op::Constant) -> HebiResult<()> {
    todo!()
  }

  fn op_make_class(&mut self, desc: op::Constant, parts: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_make_class_derived(&mut self, desc: op::Constant, parts: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_make_list(&mut self, start: op::Register, count: op::Count) -> HebiResult<()> {
    let list = List::with_capacity(count.value());
    for reg in start.iter(count, 1) {
      list.push(self.get_register(reg));
    }
    self.acc = Value::object(self.cx.alloc(list));
    Ok(())
  }

  fn op_make_list_empty(&mut self) -> HebiResult<()> {
    self.acc = Value::object(self.cx.alloc(List::new()));
    Ok(())
  }

  fn op_make_table(&mut self, start: op::Register, count: op::Count) -> HebiResult<()> {
    let table = Table::with_capacity(count.value());
    for reg in start.iter(count, 2) {
      let key = self.get_register(reg);
      let value = self.get_register(reg.offset(1));

      let Some(key) = key.clone().to_object().and_then(|v| v.cast::<String>().ok()) else {
        fail!( 0..0, "`{key}` is not a string");
      };

      table.insert(key, value);
    }
    self.acc = Value::object(self.cx.alloc(table));
    Ok(())
  }

  fn op_make_table_empty(&mut self) -> HebiResult<()> {
    self.acc = Value::object(self.cx.alloc(Table::new()));
    Ok(())
  }

  fn op_jump(&mut self, offset: op::Offset) -> HebiResult<op::Offset> {
    Ok(offset)
  }

  fn op_jump_const(&mut self, idx: op::Constant) -> HebiResult<op::Offset> {
    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };
    Ok(offset)
  }

  fn op_jump_loop(&mut self, offset: op::Offset) -> HebiResult<op::Offset> {
    Ok(offset)
  }

  fn op_jump_if_false(&mut self, offset: op::Offset) -> HebiResult<super::dispatch::Jump> {
    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Skip),
      false => Ok(super::dispatch::Jump::Move(offset)),
    }
  }

  fn op_jump_if_false_const(&mut self, idx: op::Constant) -> HebiResult<super::dispatch::Jump> {
    let offset = self.get_constant(idx).as_offset().cloned();
    debug_assert!(offset.is_some());
    let offset = unsafe { offset.unwrap_unchecked() };

    match is_truthy(take(&mut self.acc)) {
      true => Ok(super::dispatch::Jump::Move(offset)),
      false => Ok(super::dispatch::Jump::Skip),
    }
  }

  fn op_add(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_sub(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_mul(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_div(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_rem(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_pow(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_inv(&mut self) -> HebiResult<()> {
    todo!()
  }

  fn op_not(&mut self) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_eq(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_ne(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_gt(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_ge(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_lt(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_le(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_cmp_type(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_contains(&mut self, lhs: op::Register) -> HebiResult<()> {
    todo!()
  }

  fn op_is_none(&mut self) -> HebiResult<()> {
    self.acc = Value::bool(self.acc.is_none());
    Ok(())
  }

  fn op_print(&mut self) -> HebiResult<()> {
    // TODO: allow setting output writer
    println!("{}", self.acc);
    Ok(())
  }

  fn op_print_n(&mut self, start: op::Register, count: op::Count) -> HebiResult<()> {
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
  ) -> HebiResult<dispatch::LoadFrame> {
    let f = self.get_register(callee);
    let start = self.stack_base + callee.index() + 1;
    let (stack_base, num_args) = push_args!(self, f.clone(), range(start, start + args.value()));
    self.prepare_call(f, stack_base, num_args, Some(return_addr))?;
    Ok(dispatch::LoadFrame {
      bytecode: current_call_frame!(self).instructions,
      pc: self.pc,
    })
  }

  fn op_call0(&mut self, return_addr: usize) -> HebiResult<dispatch::LoadFrame> {
    let f = take(&mut self.acc);
    let stack_base = self.stack.len();
    self.prepare_call(f, stack_base, 0, Some(return_addr))?;
    Ok(dispatch::LoadFrame {
      bytecode: current_call_frame!(self).instructions,
      pc: self.pc,
    })
  }

  fn op_import(&mut self, path: op::Constant, dst: op::Register) -> HebiResult<()> {
    let path = self.get_constant_object::<String>(path);
    let module = self.load_module(path)?;
    self.set_register(dst, Value::object(module));

    Ok(())
  }

  fn op_return(&mut self) -> HebiResult<dispatch::Return> {
    // return value is in the accumulator

    // pop frame
    let frame = self.call_frames.pop().unwrap();

    // truncate stack
    self.stack.truncate(self.stack.len() - frame.frame_size);

    Ok(match self.call_frames.last() {
      Some(current_frame) => {
        self.stack_base -= current_frame.frame_size;
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

  fn op_yield(&mut self) -> HebiResult<()> {
    todo!()
  }
}
