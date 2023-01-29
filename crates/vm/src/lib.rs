mod binop;
mod call;
mod cmp;
mod error;
mod truth;
mod util;

// TODO: make the VM panic-less (other than debug asserts for unsafe things)

pub use error::Error;
use value::object::{dict, Closure, Dict};
use value::Value;

pub struct Isolate<Io: std::io::Write + Sized = std::io::Stdout> {
  // TODO: module registry
  globals: Dict,
  pc: usize,
  acc: Value,
  stack: Vec<Value>,
  call_stack: Vec<call::CallFrame>,
  io: Io,
}

impl Isolate<std::io::Stdout> {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Isolate<std::io::Stdout> {
    Isolate::<std::io::Stdout>::with_io(std::io::stdout())
  }
}

impl<Io: std::io::Write> Isolate<Io> {
  pub fn with_io(io: Io) -> Isolate<Io> {
    Isolate {
      globals: Dict::new(),
      pc: 0,
      acc: Value::none(),
      stack: vec![],
      call_stack: vec![],
      io,
    }
  }

  pub(crate) fn get_io(&self) -> &Io {
    &self.io
  }
}

impl<Io: std::io::Write> op::Handler for Isolate<Io> {
  type Error = Error;

  fn op_load_const(&mut self, slot: u32) -> Result<(), Self::Error> {
    let slot = slot as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };

    self.acc = const_pool[slot].clone();

    Ok(())
  }

  fn op_load_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let reg = reg as usize;

    self.acc = self.stack[base + reg].clone();

    Ok(())
  }

  fn op_store_reg(&mut self, reg: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let reg = reg as usize;

    self.stack[base + reg] = self.acc.clone();

    Ok(())
  }

  fn op_load_capture(&mut self, slot: u32) -> Result<(), Self::Error> {
    let slot = slot as usize;
    let captures = unsafe {
      self
        .call_stack
        .last()
        .unwrap()
        .captures
        .map(|ptr| ptr.as_ref())
    }
    .expect("attempted to load capture in function which is not a closure");

    self.acc = captures[slot].clone();

    Ok(())
  }

  fn op_store_capture(&mut self, slot: u32) -> Result<(), Self::Error> {
    let slot = slot as usize;

    let captures = unsafe {
      self
        .call_stack
        .last_mut()
        .unwrap()
        .captures
        .map(|mut ptr| ptr.as_mut())
    }
    .expect("attempted to store capture in function which is not a closure");

    captures[slot] = self.acc.clone();

    Ok(())
  }

  fn op_load_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = name as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let name = const_pool[name].clone();
    // global name is always a string
    let name = dict::Key::try_from(name).unwrap();
    match self.globals.get(name.clone()) {
      Some(v) => self.acc = v.clone(),
      // TODO: span
      None => return Err(Error::new(format!("undefined global {name}"))),
    }

    Ok(())
  }

  fn op_store_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = name as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let name = const_pool[name].clone();
    // global name is always a string
    let name = dict::Key::try_from(name).unwrap();
    self.globals.insert(name, self.acc.clone());

    Ok(())
  }

  fn op_load_named(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = name as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let name = const_pool[name].clone();
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    let value = {
      // TODO: class
      let Some(obj) = self.acc.as_dict() else {
        // TODO: span
        return Err(Error::new(format!("undefined field {name}")));
      };

      let Some(value) = obj.get(name.clone()) else {
        // TODO: span
        return Err(Error::new(format!("undefined field {name}")));
      };

      value.clone()
    };

    self.acc = value;

    Ok(())
  }

  fn op_load_named_opt(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = name as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let name = const_pool[name].clone();
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    // early exit if on `none`
    if self.acc.is_none() {
      return Ok(());
    }

    let value = {
      // TODO: class
      let Some(obj) = self.acc.as_dict() else {
        // TODO: span
        return Err(Error::new(format!("undefined field {name}")));
      };

      match obj.get(name) {
        Some(v) => v.clone(),
        None => Value::none(),
      }
    };

    self.acc = value;

    Ok(())
  }

  fn op_store_named(&mut self, name: u32, obj: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let name = name as usize;
    let obj = obj as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let name = const_pool[name].clone();
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    // TODO: class
    let Some(mut obj) = self.stack[base + obj].as_dict_mut() else {
      // TODO: span
      return Err(Error::new("value is not an object"));
    };

    obj.insert(name, self.acc.clone());

    Ok(())
  }

  fn op_load_keyed(&mut self, key: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let key = key as usize;

    let name = self.stack[base + key].clone();
    let Ok(name) = dict::Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key")));
    };

    let value = {
      // TODO: class
      let Some(obj) = self.acc.as_dict() else {
        // TODO: span
        return Err(Error::new(format!("undefined field {name}")));
      };

      let Some(value) = obj.get(name.clone()) else {
        // TODO: span
        return Err(Error::new(format!("undefined field {name}")));
      };

      value.clone()
    };

    self.acc = value;

    Ok(())
  }

  fn op_load_keyed_opt(&mut self, key: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let key = key as usize;

    let name = self.stack[base + key].clone();
    let Ok(name) = dict::Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key")));
    };

    // early exit if on `none`
    if self.acc.is_none() {
      return Ok(());
    }

    let value = {
      // TODO: class
      let Some(obj) = self.acc.as_dict() else {
        // TODO: span
        return Err(Error::new(format!("undefined field {name}")));
      };

      match obj.get(name) {
        Some(v) => v.clone(),
        None => Value::none(),
      }
    };

    self.acc = value;

    Ok(())
  }

  fn op_store_keyed(&mut self, key: u32, obj: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let key = key as usize;
    let obj = obj as usize;

    let name = self.stack[base + key].clone();
    let Ok(name) = dict::Key::try_from(name.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{name} is not a valid key")));
    };

    // TODO: class
    let Some(mut obj) = self.stack[base + obj].as_dict_mut() else {
      // TODO: span
      return Err(Error::new("value is not an object"));
    };

    obj.insert(name, self.acc.clone());

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
    let base = self.call_stack.last().unwrap().stack_base;
    let list = list as usize;

    let Some(mut list) = self.stack[base + list].as_list_mut() else {
      // TODO: span
      return Err(Error::new("value is not a list"));
    };

    list.push(self.acc.clone());

    Ok(())
  }

  fn op_create_empty_dict(&mut self) -> Result<(), Self::Error> {
    self.acc = Dict::new().into();

    Ok(())
  }

  fn op_insert_to_dict(&mut self, key: u32, dict: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let key = key as usize;
    let dict = dict as usize;

    let key = self.stack[base + key].clone();
    let Ok(key) = dict::Key::try_from(key.clone()) else {
      // TODO: span
      return Err(Error::new(format!("{key} is not a valid key")));
    };

    let Some(mut dict) = self.stack[base + dict].as_dict_mut() else {
      // TODO: span
      return Err(Error::new("value is not an object"));
    };

    // `name` is a `Key` so this `unwrap` won't panic
    dict.insert(key, self.acc.clone());

    Ok(())
  }

  fn op_insert_to_dict_named(&mut self, name: u32, dict: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let name = name as usize;
    let dict = dict as usize;

    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let name = const_pool[name].clone();
    // name used in named load is always a string
    let name = dict::Key::try_from(name).unwrap();

    let Some(mut dict) = self.stack[base + dict].as_dict_mut() else {
      // TODO: span
      return Err(Error::new("value is not an object"));
    };

    // name used in named load is always a string
    dict.insert(name, self.acc.clone());

    Ok(())
  }

  fn op_create_closure(&mut self, descriptor: u32) -> Result<(), Self::Error> {
    let descriptor = descriptor as usize;
    let const_pool = unsafe { self.call_stack.last().unwrap().const_pool.as_ref() };
    let descriptor = const_pool[descriptor].clone();

    self.acc = Closure::new(descriptor).into();

    Ok(())
  }

  fn op_capture_reg(&mut self, reg: u32, slot: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let reg = reg as usize;
    let slot = slot as usize;

    // should not panic as long as bytecode is valid
    let captures = &mut self
      .acc
      .as_closure_mut()
      .expect("attempted to capture register for value which is not a closure")
      .captures;

    captures[slot] = self.stack[base + reg].clone();

    Ok(())
  }

  fn op_capture_slot(&mut self, parent_slot: u32, self_slot: u32) -> Result<(), Self::Error> {
    let parent_slot = parent_slot as usize;
    let self_slot = self_slot as usize;

    let parent_captures = unsafe {
      self
        .call_stack
        .last_mut()
        .unwrap()
        .captures
        .map(|mut ptr| ptr.as_mut())
    }
    .expect("attempted to store capture in function which is not a closure");

    // should not panic as long as bytecode is valid
    let self_captures = &mut self
      .acc
      .as_closure_mut()
      .expect("attempted to capture register for value which is not a closure")
      .captures;

    self_captures[self_slot] = parent_captures[parent_slot].clone();

    Ok(())
  }

  fn op_jump(&mut self, offset: u32) -> Result<op::ControlFlow, Self::Error> {
    Ok(op::ControlFlow::Jump(offset))
  }

  fn op_jump_back(&mut self, offset: u32) -> Result<op::ControlFlow, Self::Error> {
    Ok(op::ControlFlow::Loop(offset))
  }

  fn op_jump_if_false(&mut self, offset: u32) -> Result<op::ControlFlow, Self::Error> {
    let Some(value) = self.acc.as_bool() else {
      // TODO: span
      return Err(Error::new("value is not a bool"));
    };

    match value {
      true => Ok(op::ControlFlow::Next),
      false => Ok(op::ControlFlow::Jump(offset)),
    }
  }

  fn op_add(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    match binop::add(lhs, rhs) {
      Ok(value) => self.acc = value,
      Err(e) => return Err(e),
    }

    Ok(())
  }

  fn op_sub(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    match binop::sub(lhs, rhs) {
      Ok(value) => self.acc = value,
      Err(e) => return Err(e),
    }

    Ok(())
  }

  fn op_mul(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    match binop::mul(lhs, rhs) {
      Ok(value) => self.acc = value,
      Err(e) => return Err(e),
    }

    Ok(())
  }

  fn op_div(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    match binop::div(lhs, rhs) {
      Ok(value) => self.acc = value,
      Err(e) => return Err(e),
    }

    Ok(())
  }

  fn op_rem(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    match binop::rem(lhs, rhs) {
      Ok(value) => self.acc = value,
      Err(e) => return Err(e),
    }

    Ok(())
  }

  fn op_pow(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    match binop::pow(lhs, rhs) {
      Ok(value) => self.acc = value,
      Err(e) => return Err(e),
    }

    Ok(())
  }

  fn op_unary_plus(&mut self) -> Result<(), Self::Error> {
    // TODO: convert to number (with overload)
    // does nothing for now

    Ok(())
  }

  fn op_unary_minus(&mut self) -> Result<(), Self::Error> {
    let value = self.acc.clone();
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
    let value = !truth::truthiness(self.acc.clone());

    self.acc = Value::bool(value);

    Ok(())
  }

  fn op_cmp_eq(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    let ord = match cmp::partial_cmp(lhs, rhs) {
      Ok(v) => v,
      Err(e) => return Err(e),
    };

    self.acc = Value::bool(matches!(ord, Some(cmp::Ordering::Equal)));

    Ok(())
  }

  fn op_cmp_neq(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    let ord = match cmp::partial_cmp(lhs, rhs) {
      Ok(v) => v,
      Err(e) => return Err(e),
    };

    self.acc = Value::bool(!matches!(ord, Some(cmp::Ordering::Equal)));

    Ok(())
  }

  fn op_cmp_gt(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    let ord = match cmp::partial_cmp(lhs, rhs) {
      Ok(v) => v,
      Err(e) => return Err(e),
    };

    self.acc = Value::bool(matches!(ord, Some(cmp::Ordering::Greater)));

    Ok(())
  }

  fn op_cmp_ge(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    let ord = match cmp::partial_cmp(lhs, rhs) {
      Ok(v) => v,
      Err(e) => return Err(e),
    };

    self.acc = Value::bool(matches!(
      ord,
      Some(cmp::Ordering::Greater | cmp::Ordering::Equal)
    ));

    Ok(())
  }

  fn op_cmp_lt(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    let ord = match cmp::partial_cmp(lhs, rhs) {
      Ok(v) => v,
      Err(e) => return Err(e),
    };

    self.acc = Value::bool(matches!(ord, Some(cmp::Ordering::Less)));

    Ok(())
  }

  fn op_cmp_le(&mut self, lhs: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let lhs = lhs as usize;

    // TODO: object overload
    let lhs = self.stack[base + lhs].clone();
    let rhs = self.acc.clone();

    let ord = match cmp::partial_cmp(lhs, rhs) {
      Ok(v) => v,
      Err(e) => return Err(e),
    };

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
    let value = &self.acc;
    self
      .io
      .write_fmt(format_args!("{value}"))
      // TODO: span
      .map_err(|_| Error::new("failed to print value"))?;
    Ok(())
  }

  fn op_print_list(&mut self, list: u32) -> Result<(), Self::Error> {
    let base = self.call_stack.last().unwrap().stack_base;
    let list = list as usize;

    let list = self.stack[base + list].clone();
    let list = list.as_list().expect("print_list argument is not a list");

    // prints items separated by a single space
    let mut iter = list.iter().peekable();
    while let Some(value) = iter.next() {
      if iter.peek().is_some() {
        // space at end
        self
          .io
          .write_fmt(format_args!("{value} "))
          // TODO: span
          .map_err(|_| Error::new("failed to print values"))?;
      } else {
        self
          .io
          .write_fmt(format_args!("{value}"))
          // TODO: span
          .map_err(|_| Error::new("failed to print values"))?;
      }
    }

    Ok(())
  }

  fn op_call(&mut self, callee: u32, args: u32) -> Result<(), Self::Error> {
    unimplemented!()
  }

  fn op_call_kw(&mut self, callee: u32, args: u32) -> Result<(), Self::Error> {
    unimplemented!()
  }

  fn op_is_pos_param_not_set(&mut self, index: u32) -> Result<(), Self::Error> {
    unimplemented!()
  }

  fn op_is_kw_param_not_set(&mut self, name: u32) -> Result<(), Self::Error> {
    unimplemented!()
  }

  fn op_load_kw_param(&mut self, name: u32, param: u32) -> Result<(), Self::Error> {
    unimplemented!()
  }

  fn op_ret(&mut self) -> Result<(), Self::Error> {
    unimplemented!()
  }
}

#[cfg(test)]
mod tests;
