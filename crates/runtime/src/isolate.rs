// TODO: make the VM panic-less (other than debug asserts for unsafe things)
// TODO: module registry
// TODO: store the reserved stack slots as constants somewhere (??)

mod binop;
mod call;
mod class;
mod cmp;
mod field;
mod index;
mod string;
mod truth;

use std::mem::take;
use std::ptr::NonNull;

use crate::value::object::frame::{Frame, Stack};
use crate::value::object::handle::Handle;
use crate::value::object::{
  frame, ClassDef, Closure, Dict, Key, List, ObjectType, Proxy, Registry,
};
use crate::value::Value;
use crate::Error;

// TODO: fields should be private, even to ops
pub struct Isolate<Io: std::io::Write + Sized = std::io::Stdout> {
  registry: Handle<Registry>,
  globals: Handle<Dict>,

  width: op::Width,
  pc: usize,

  acc: Value,
  frames: Vec<Frame>,
  current_frame: Option<NonNull<Frame>>,
  io: Io,
}

impl Isolate<std::io::Stdout> {
  pub fn new(registry: Handle<Registry>) -> Isolate<std::io::Stdout> {
    Isolate::<std::io::Stdout>::with_io(registry, std::io::stdout())
  }
}

impl<Io: std::io::Write> Isolate<Io> {
  pub fn with_io(registry: Handle<Registry>, io: Io) -> Isolate<Io> {
    Isolate {
      registry,
      globals: Handle::alloc(Dict::new()),

      width: op::Width::Single,
      pc: 0,

      acc: Value::none(),
      frames: vec![],
      current_frame: None,
      io,
    }
  }

  pub fn alloc<T: ObjectType>(&self, v: T) -> Handle<T> {
    Handle::alloc(v)
  }

  pub fn io(&self) -> &Io {
    &self.io
  }

  pub fn print(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    self.io.write_fmt(args)
  }

  fn push_frame(&mut self, frame: Frame) {
    self.frames.push(frame);
    self.current_frame = Some(NonNull::from(unsafe {
      self.frames.last_mut().unwrap_unchecked()
    }));
  }

  fn pop_frame(&mut self) -> Frame {
    let frame = self.frames.pop().expect("call stack underflow");
    self.current_frame = self.frames.last_mut().map(NonNull::from);
    frame
  }

  fn current_frame(&self) -> &Frame {
    unsafe { &*self.current_frame.unwrap().as_ptr() }
  }

  fn current_frame_mut(&mut self) -> &mut Frame {
    unsafe { &mut *self.current_frame.unwrap().as_ptr() }
  }

  fn get_const(&self, slot: u32) -> Value {
    let frame = self.current_frame();
    let const_pool = unsafe { frame.const_pool.as_ref() };
    const_pool[slot as usize].clone().into()
  }

  fn get_reg(&self, index: u32) -> Value {
    let frame = self.current_frame();
    frame.stack[index as usize].clone()
  }

  fn set_reg(&mut self, index: u32, value: Value) {
    let frame = self.current_frame_mut();
    frame.stack[index as usize] = value;
  }

  fn get_capture(&self, slot: u32) -> Value {
    let frame = self.current_frame();
    let captures = unsafe { frame.captures.unwrap().as_ref() };
    captures[slot as usize].clone()
  }

  fn set_capture(&mut self, slot: u32, value: Value) {
    let frame = self.current_frame_mut();
    unsafe { frame.captures.unwrap().as_mut()[slot as usize] = value }
  }

  fn stack(&self) -> &Stack {
    &self.current_frame().stack
  }
}

pub enum Control {
  Error(Error),
  Yield,
}

impl From<Error> for Control {
  fn from(value: Error) -> Self {
    Self::Error(value)
  }
}

impl<Io: std::io::Write> op::Handler for Isolate<Io> {
  type Error = Control;

  fn op_load_const(&mut self, slot: u32) -> Result<(), Self::Error> {
    self.acc = self.get_const(slot);

    Ok(())
  }

  fn op_load_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
    self.acc = self.get_reg(reg);

    Ok(())
  }

  fn op_store_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
    self.set_reg(reg, self.acc.clone());

    Ok(())
  }

  fn op_load_capture(&mut self, slot: u32) -> Result<(), Self::Error> {
    self.acc = self.get_capture(slot);

    Ok(())
  }

  fn op_store_capture(&mut self, slot: u32) -> Result<(), Self::Error> {
    self.set_capture(slot, self.acc.clone());

    Ok(())
  }

  fn op_load_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // global name is always a string
    let name = Key::try_from(name).unwrap();
    match self.globals.get(&name) {
      Some(v) => self.acc = v.clone(),
      // TODO: span
      None => return Err(Error::new(format!("undefined global {name}"), 0..0).into()),
    }

    Ok(())
  }

  fn op_store_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // global name is always a string
    let name = Key::try_from(name).unwrap();
    self.globals.insert(name, self.acc.clone());

    Ok(())
  }

  fn op_load_field(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = Key::try_from(name).unwrap();
    let name = name.as_str().unwrap();

    self.acc = field::get(&self.acc, name)?;

    Ok(())
  }

  fn op_load_field_opt(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = Key::try_from(name).unwrap();
    let name = name.as_str().unwrap();

    self.acc = field::get_opt(&self.acc, name)?;

    Ok(())
  }

  fn op_store_field(&mut self, name: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = Key::try_from(name).unwrap();
    let name = name.as_str().unwrap();

    let mut obj = self.get_reg(obj);

    field::set(&mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_load_index(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key"), 0..0).into());
    };

    self.acc = index::get(&self.acc, &name)?;

    Ok(())
  }

  fn op_load_index_opt(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key"), 0..0).into());
    };

    self.acc = index::get_opt(&self.acc, &name)?;

    Ok(())
  }

  fn op_store_index(&mut self, key: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key"), 0..0).into());
    };

    let mut obj = self.get_reg(obj);

    index::set(&mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_load_module(&mut self, path: u32, dest: u32) -> Result<(), Self::Error> {
    todo!()
  }

  fn op_load_self(&mut self) -> Result<(), Self::Error> {
    // receiver is always placed at the base of the current call frame's stack
    let this = self.get_reg(3);

    if let Some(proxy) = this.clone().to_proxy() {
      self.acc = Value::object(proxy.class());
      return Ok(());
    }

    assert!(this.is_class());
    self.acc = this;

    Ok(())
  }

  fn op_load_super(&mut self) -> Result<(), Self::Error> {
    // receiver is always placed at the base of the current call frame's stack
    let this = self.get_reg(3);

    // all parent class `unwrap()`s here should never panic,
    // because parser checks for parent class

    if let Some(proxy) = this.clone().to_proxy() {
      // we're in a super class
      // proxy to the next super class in the chain
      self.acc = Value::object(self.alloc(Proxy::new(
        proxy.class(),
        //    current  next
        //    |        |
        proxy.parent().parent().unwrap(),
      )));
      return Ok(());
    }

    // we're not in a super class yet,
    // proxy to the first super class in the chain
    let Some(this) = this.to_class() else {
      // TODO: span
      return Err(Error::new("receiver is not a class", 0..0).into());
    };
    let parent = this.parent().unwrap();
    self.acc = Value::object(Handle::alloc(Proxy::new(this, parent)));

    Ok(())
  }

  fn op_push_none(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::none();

    Ok(())
  }

  fn op_push_true(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::bool(true);

    Ok(())
  }

  fn op_push_false(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::bool(false);

    Ok(())
  }

  fn op_push_small_int(&mut self, value: i32) -> Result<(), Self::Error> {
    self.acc = Value::int(value);

    Ok(())
  }

  fn op_create_empty_list(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::object(Handle::alloc(List::new()));

    Ok(())
  }

  fn op_push_to_list(&mut self, list: u32) -> Result<(), Self::Error> {
    let list = self.get_reg(list);

    let Some(mut list) = list.to_list() else {
      // TODO: span
      return Err(Error::new("value is not a list", 0..0).into());
    };

    list.push(take(&mut self.acc));

    Ok(())
  }

  fn op_create_empty_dict(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::object(Handle::alloc(Dict::new()));

    Ok(())
  }

  fn op_insert_to_dict(&mut self, key: u32, dict: u32) -> Result<(), Self::Error> {
    let key = self.get_reg(key);
    let Ok(key) = Key::try_from(key.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{key} is not a valid key"), 0..0).into());
    };

    let dict = self.get_reg(dict);
    let mut dict = dict.to_dict().unwrap();

    // `name` is a `Key` so this `unwrap` won't panic
    dict.insert(key, take(&mut self.acc));

    Ok(())
  }

  fn op_insert_to_dict_named(&mut self, name: u32, dict: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = Key::try_from(name).unwrap();

    let dict = self.get_reg(dict);
    let mut dict = dict.to_dict().unwrap();

    // name used in named load is always a string
    dict.insert(name, take(&mut self.acc));

    Ok(())
  }

  fn op_create_closure(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);

    // this should always be a closure descriptor
    let desc = desc.to_closure_desc().unwrap();

    self.acc = Value::object(Handle::alloc(Closure::new(desc)));

    Ok(())
  }

  fn op_capture_reg(&mut self, reg: u32, slot: u32) -> Result<(), Self::Error> {
    let value = self.get_reg(reg);

    // this should always be a closure
    let mut closure = self.acc.clone().to_closure().unwrap();

    let captures = unsafe { closure.captures_mut() };
    captures[slot as usize] = value;

    Ok(())
  }

  fn op_capture_slot(&mut self, parent_slot: u32, self_slot: u32) -> Result<(), Self::Error> {
    let value = self.get_capture(parent_slot);

    // should not panic as long as bytecode is valid
    let mut closure = self.acc.clone().to_closure().unwrap();

    let captures = unsafe { closure.captures_mut() };
    captures[self_slot as usize] = value;

    Ok(())
  }

  fn op_create_class_empty(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = desc.to_class_desc().unwrap();

    self.acc = Value::object(Handle::alloc(ClassDef::new(desc, &[])));

    Ok(())
  }

  fn op_create_class(&mut self, desc: u32, start: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = desc.to_class_desc().unwrap();

    let value = Value::object(Handle::alloc(ClassDef::new(
      desc,
      &self.stack()[start as usize..],
    )));
    self.acc = value;

    Ok(())
  }

  fn op_jump(&mut self, offset: u32) -> Result<op::ControlFlow, Self::Error> {
    Ok(op::ControlFlow::Jump(offset as usize))
  }

  fn op_jump_back(&mut self, offset: u32) -> Result<op::ControlFlow, Self::Error> {
    Ok(op::ControlFlow::Loop(offset as usize))
  }

  fn op_jump_if_false(&mut self, offset: u32) -> Result<op::ControlFlow, Self::Error> {
    let Some(value) = self.acc.clone().to_bool() else {
      // TODO: span
      return Err(Error::new("value is not a bool", 0..0).into());
    };

    match value {
      true => Ok(op::ControlFlow::Nop),
      false => Ok(op::ControlFlow::Jump(offset as usize)),
    }
  }

  fn op_add(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    self.acc = binop::add(lhs, rhs)?;

    Ok(())
  }

  fn op_sub(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    self.acc = binop::sub(lhs, rhs)?;

    Ok(())
  }

  fn op_mul(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    self.acc = binop::mul(lhs, rhs)?;

    Ok(())
  }

  fn op_div(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    self.acc = binop::div(lhs, rhs)?;

    Ok(())
  }

  fn op_rem(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    self.acc = binop::rem(lhs, rhs)?;

    Ok(())
  }

  fn op_pow(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    self.acc = binop::pow(lhs, rhs)?;

    Ok(())
  }

  fn op_unary_plus(&mut self) -> Result<(), Self::Error> {
    // TODO: convert to number (with overload)
    // does nothing for now

    Ok(())
  }

  fn op_unary_minus(&mut self) -> Result<(), Self::Error> {
    let value = take(&mut self.acc);
    let value = if let Some(value) = value.clone().to_int() {
      Value::int(-value)
    } else if let Some(value) = value.to_float() {
      Value::float(-value)
    } else {
      // TODO: overload
      unimplemented!()
    };

    self.acc = value;

    Ok(())
  }

  fn op_unary_not(&mut self) -> Result<(), Self::Error> {
    // TODO: overload
    let value = !truth::truthiness(take(&mut self.acc));

    self.acc = Value::bool(value);

    Ok(())
  }

  fn op_cmp_eq(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = Value::bool(matches!(ord, Some(cmp::Ordering::Equal)));

    Ok(())
  }

  fn op_cmp_neq(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = Value::bool(!matches!(ord, Some(cmp::Ordering::Equal)));

    Ok(())
  }

  fn op_cmp_gt(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = Value::bool(matches!(ord, Some(cmp::Ordering::Greater)));

    Ok(())
  }

  fn op_cmp_ge(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = Value::bool(matches!(
      ord,
      Some(cmp::Ordering::Greater | cmp::Ordering::Equal)
    ));

    Ok(())
  }

  fn op_cmp_lt(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = Value::bool(matches!(ord, Some(cmp::Ordering::Less)));

    Ok(())
  }

  fn op_cmp_le(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = Value::bool(matches!(
      ord,
      Some(cmp::Ordering::Equal | cmp::Ordering::Less)
    ));

    Ok(())
  }

  fn op_is_none(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::bool(self.acc.is_none());

    Ok(())
  }

  fn op_print(&mut self) -> Result<(), Self::Error> {
    let value = take(&mut self.acc);
    self
      .print(format_args!("{}\n", string::stringify(value)))
      // TODO: span
      .map_err(|_| Error::new("failed to print value", 0..0))?;
    Ok(())
  }

  fn op_print_list(&mut self, list: u32) -> Result<(), Self::Error> {
    let list = self.get_reg(list);
    let list = list.to_list().unwrap();

    // print is a statement so should not leave a value in `acc`
    let _ = take(&mut self.acc);

    // prints items separated by a single space
    let mut iter = list.iter().peekable();
    while let Some(value) = iter.next() {
      if iter.peek().is_some() {
        // space at end
        self
          .print(format_args!("{} ", string::stringify(value.clone())))
          // TODO: span
          .map_err(|_| Error::new("failed to print values", 0..0))?;
      } else {
        self
          .print(format_args!("{}", string::stringify(value.clone())))
          // TODO: span
          .map_err(|_| Error::new("failed to print values", 0..0))?;
      }
    }
    self
      .print(format_args!("\n"))
      // TODO: span
      .map_err(|_| Error::new("failed to print values", 0..0))?;

    Ok(())
  }

  fn op_call0(&mut self, return_address: usize) -> Result<(), Self::Error> {
    let callable = self.acc.clone();

    if callable.is_class_def() {
      // class constructor
      let class_def = callable.to_class_def().unwrap();
      self.acc = class::create_instance(self, class_def, &[], Value::none())?;
      self.pc = return_address;
      return Ok(());
    }

    // regular function call
    let frame = self.prepare_call_frame(
      callable,
      &[],
      Value::none(),
      frame::OnReturn::Jump(return_address),
    )?;
    self.push_frame(frame);
    self.pc = 0;
    Ok(())
  }

  fn op_call(&mut self, start: u32, args: u32, return_address: usize) -> Result<(), Self::Error> {
    let callable = self.acc.clone();
    // TODO: remove `to_vec` somehow
    let args = self.stack()[start as usize..][..args as usize].to_vec();

    if callable.is_class_def() {
      // class constructor
      let class_def = callable.to_class_def().unwrap();
      self.acc = class::create_instance(self, class_def, &args, Value::none())?;
      self.pc = return_address;
      return Ok(());
    }

    // regular function call
    let frame = self.prepare_call_frame(
      callable,
      &args,
      Value::none(),
      frame::OnReturn::Jump(return_address),
    )?;
    self.push_frame(frame);
    self.width = op::Width::Single;
    self.pc = 0;
    Ok(())
  }

  fn op_call_kw(
    &mut self,
    start: u32,
    args: u32,
    return_address: usize,
  ) -> Result<(), Self::Error> {
    let callable = self.acc.clone();
    let kwargs = self.get_reg(start);
    // TODO: remove `to_vec` somehow
    let args = self.stack()[start as usize + 1..][..args as usize].to_vec();

    if callable.is_class_def() {
      // class constructor
      let def = callable.to_class_def().unwrap();
      self.acc = class::create_instance(self, def, &args, kwargs)?;
      self.pc = return_address;
      return Ok(());
    }

    // regular function call
    let frame = self.prepare_call_frame(
      callable,
      &args,
      kwargs,
      frame::OnReturn::Jump(return_address),
    )?;
    self.push_frame(frame);
    self.width = op::Width::Single;
    self.pc = 0;
    Ok(())
  }

  fn op_is_pos_param_not_set(&mut self, index: u32) -> Result<(), Self::Error> {
    let frame = self.current_frame();
    let index = index as usize;

    self.acc = Value::bool(frame.num_args <= index);

    Ok(())
  }

  fn op_is_kw_param_not_set(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name is always a string here
    let name = Key::try_from(name).unwrap();
    // base + 3 is always the kw dictionary
    let kwargs = self.get_reg(2);
    let kwargs = kwargs.to_dict().unwrap();

    self.acc = Value::bool(!kwargs.contains_key(&name));

    Ok(())
  }

  fn op_load_kw_param(&mut self, name: u32, param: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name is always a string here
    let name = Key::try_from(name).unwrap();
    // base + 3 is always the kw dictionary
    let kwargs = self.get_reg(2);
    let mut kwargs = kwargs.to_dict().unwrap();

    self.set_reg(param, kwargs.remove(&name).unwrap());

    Ok(())
  }

  fn op_ret(&mut self) -> Result<(), Self::Error> {
    match self.pop_frame().on_return {
      frame::OnReturn::Jump(offset) => {
        self.pc = offset;
        Ok(())
      }
      frame::OnReturn::Yield => Err(Control::Yield),
    }
  }
}
