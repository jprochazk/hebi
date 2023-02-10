mod binop;
mod call;
mod class;
mod cmp;
mod error;
mod field;
mod truth;
mod util;

// TODO: make the VM panic-less (other than debug asserts for unsafe things)
// TODO: stack unwinding
// TODO: module registry
// TODO: store the reserved stack slots as constants somewhere (??)

use std::mem::take;
use std::ptr::NonNull;

pub use error::Error;
use value::object::frame::{Frame, Stack};
use value::object::handle::Handle;
use value::object::{
  dict, frame, Class, ClassDef, ClassDesc, Closure, ClosureDesc, Dict, Proxy, Registry,
};
use value::Value;

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
      globals: Dict::new().into(),

      width: op::Width::Single,
      pc: 0,

      acc: Value::none(),
      frames: vec![],
      current_frame: None,
      io,
    }
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
    unsafe { frame.const_pool.as_ref()[slot as usize].clone() }
  }

  fn get_reg(&self, index: u32) -> Value {
    let frame = self.current_frame();
    let value = frame.stack.get(index as usize).clone();
    value
  }

  fn set_reg(&mut self, index: u32, value: Value) {
    let mut frame = self.current_frame_mut();
    frame.stack.set(index as usize, value);
  }

  fn get_capture(&self, slot: u32) -> Value {
    let frame = self.current_frame();
    unsafe { frame.captures.unwrap().as_ref()[slot as usize].clone() }
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
    let name = dict::Key::try_from(name).unwrap();
    match self.globals.borrow().get(&name) {
      Some(v) => self.acc = v.clone(),
      // TODO: span
      None => return Err(Error::new(format!("undefined global {name}")).into()),
    }

    Ok(())
  }

  fn op_store_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // global name is always a string
    let name = dict::Key::try_from(name).unwrap();
    self.globals.borrow_mut().insert(name, self.acc.clone());

    Ok(())
  }

  fn op_load_named(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    self.acc = field::get(&self.acc, &name)?;

    Ok(())
  }

  fn op_load_named_opt(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    self.acc = field::get_opt(&self.acc, &name)?;

    Ok(())
  }

  fn op_store_named(&mut self, name: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    let mut obj = self.get_reg(obj);

    field::set(&mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_load_keyed(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = dict::Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key")).into());
    };

    self.acc = field::get(&self.acc, &name)?;

    Ok(())
  }

  fn op_load_keyed_opt(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = dict::Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key")).into());
    };

    self.acc = field::get_opt(&self.acc, &name)?;

    Ok(())
  }

  fn op_store_keyed(&mut self, key: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = dict::Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key")).into());
    };

    let mut obj = self.get_reg(obj);

    field::set(&mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_load_module(&mut self, path: u32, dest: u32) -> Result<(), Self::Error> {
    todo!()
  }

  fn op_load_self(&mut self) -> Result<(), Self::Error> {
    // receiver is always placed at the base of the current call frame's stack
    let this = self.get_reg(3);

    if let Some(proxy) = this.as_proxy() {
      self.acc = proxy.class().clone().into();
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

    if let Some(proxy) = this.as_proxy() {
      // we're in a super class
      // proxy to the next super class in the chain
      self.acc = Proxy::new(
        proxy.class().clone(),
        //    current           next
        //    |                 |
        proxy.parent().borrow().parent().unwrap().clone(),
      )
      .into();
      return Ok(());
    }

    // we're not in a super class yet,
    // proxy to the first super class in the chain
    let Some(this) = Handle::<Class>::from_value(this) else {
      // TODO: span
      return Err(Error::new("receiver is not a class").into());
    };
    let parent = this.borrow().parent().unwrap().clone();
    self.acc = Proxy::new(this, parent).into();

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
    self.acc = vec![].into();

    Ok(())
  }

  fn op_push_to_list(&mut self, list: u32) -> Result<(), Self::Error> {
    let mut list = self.get_reg(list);

    let Some(mut list) = list.as_list_mut() else {
      // TODO: span
      return Err(Error::new("value is not a list").into());
    };

    list.push(take(&mut self.acc));

    Ok(())
  }

  fn op_create_empty_dict(&mut self) -> Result<(), Self::Error> {
    self.acc = Dict::new().into();

    Ok(())
  }

  fn op_insert_to_dict(&mut self, key: u32, dict: u32) -> Result<(), Self::Error> {
    let key = self.get_reg(key);
    let Ok(key) = dict::Key::try_from(key.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{key} is not a valid key")).into());
    };

    let mut dict = self.get_reg(dict);
    let mut dict = dict.as_dict_mut().unwrap();

    // `name` is a `Key` so this `unwrap` won't panic
    dict.insert(key, take(&mut self.acc));

    Ok(())
  }

  fn op_insert_to_dict_named(&mut self, name: u32, dict: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    let mut dict = self.get_reg(dict);
    let mut dict = dict.as_dict_mut().unwrap();

    // name used in named load is always a string
    dict.insert(name, take(&mut self.acc));

    Ok(())
  }

  fn op_create_closure(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);

    // this should always be a closure descriptor
    let desc = Handle::<ClosureDesc>::from_value(desc).unwrap();

    self.acc = Closure::new(desc).into();

    Ok(())
  }

  fn op_capture_reg(&mut self, reg: u32, slot: u32) -> Result<(), Self::Error> {
    let value = self.get_reg(reg);

    // should not panic as long as bytecode is valid
    let captures = &mut self
      .acc
      .as_closure_mut()
      .expect("attempted to capture register for value which is not a closure")
      .captures;

    captures[slot as usize] = value;

    Ok(())
  }

  fn op_capture_slot(&mut self, parent_slot: u32, self_slot: u32) -> Result<(), Self::Error> {
    let value = self.get_capture(parent_slot);

    // should not panic as long as bytecode is valid
    let self_captures = &mut self
      .acc
      .as_closure_mut()
      .expect("attempted to capture register for value which is not a closure")
      .captures;

    self_captures[self_slot as usize] = value;

    Ok(())
  }

  fn op_create_class_empty(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = Handle::<ClassDesc>::from_value(desc).unwrap();

    self.acc = Value::from(ClassDef::new(desc, &[]));

    Ok(())
  }

  fn op_create_class(&mut self, desc: u32, start: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = Handle::<ClassDesc>::from_value(desc).unwrap();

    let value = Value::from(ClassDef::new(desc, &self.stack().slice(start as usize..)));
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
    let Some(value) = self.acc.as_bool() else {
      // TODO: span
      return Err(Error::new("value is not a bool").into());
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
    let value = if let Some(value) = value.as_int() {
      Value::int(-value)
    } else if let Some(value) = value.as_float() {
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
      .print(format_args!("{value}\n"))
      // TODO: span
      .map_err(|_| Error::new("failed to print value"))?;
    Ok(())
  }

  fn op_print_list(&mut self, list: u32) -> Result<(), Self::Error> {
    let list = self.get_reg(list);
    let list = list.as_list().unwrap();

    // print is a statement so should not leave a value in `acc`
    let _ = take(&mut self.acc);

    // prints items separated by a single space
    let mut iter = list.iter().peekable();
    while let Some(value) = iter.next() {
      if iter.peek().is_some() {
        // space at end
        self
          .print(format_args!("{value} "))
          // TODO: span
          .map_err(|_| Error::new("failed to print values"))?;
      } else {
        self
          .print(format_args!("{value}"))
          // TODO: span
          .map_err(|_| Error::new("failed to print values"))?;
      }
    }
    self
      .print(format_args!("\n"))
      // TODO: span
      .map_err(|_| Error::new("failed to print values"))?;

    Ok(())
  }

  fn op_call0(&mut self, return_address: usize) -> Result<(), Self::Error> {
    let func = self.acc.clone();

    if func.is_class_def() {
      // class constructor
      let def = Handle::from_value(func).unwrap();
      self.acc = class::create_instance(self, def, &[], Value::none())?;
      self.pc = return_address;
      return Ok(());
    }

    // regular function call
    let frame = self.prepare_call_frame(
      func,
      &[],
      Value::none(),
      frame::OnReturn::Jump(return_address),
    )?;
    self.push_frame(frame);
    self.pc = 0;
    Ok(())
  }

  fn op_call(&mut self, start: u32, args: u32, return_address: usize) -> Result<(), Self::Error> {
    let func = self.acc.clone();
    // TODO: remove `to_vec` somehow
    let args = self
      .stack()
      .slice(start as usize..start as usize + args as usize)
      .to_vec();

    if func.is_class_def() {
      // class constructor
      let def = Handle::from_value(func).unwrap();
      self.acc = class::create_instance(self, def, &args, Value::none())?;
      self.pc = return_address;
      return Ok(());
    }

    // regular function call
    let frame = self.prepare_call_frame(
      func,
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
    let func = self.acc.clone();
    let kwargs = self.get_reg(start);
    // TODO: remove `to_vec` somehow
    let args = self
      .stack()
      .slice(start as usize + 1..start as usize + 1 + args as usize)
      .to_vec();

    if func.is_class_def() {
      // class constructor
      let def = Handle::from_value(func).unwrap();
      self.acc = class::create_instance(self, def, &args, kwargs)?;
      self.pc = return_address;
      return Ok(());
    }

    // regular function call
    let frame =
      self.prepare_call_frame(func, &args, kwargs, frame::OnReturn::Jump(return_address))?;
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
    let name = dict::Key::try_from(name).unwrap();
    // base + 3 is always the kw dictionary
    let kwargs = self.get_reg(2);
    let kwargs = kwargs.as_dict().unwrap();

    self.acc = Value::bool(!kwargs.contains_key(&name));

    Ok(())
  }

  fn op_load_kw_param(&mut self, name: u32, param: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name is always a string here
    let name = dict::Key::try_from(name).unwrap();
    // base + 3 is always the kw dictionary
    let mut kwargs = self.get_reg(2);
    let mut kwargs = kwargs.as_dict_mut().unwrap();

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
