// TODO: make the VM panic-less (other than debug asserts for unsafe things)
// TODO: module registry
// TODO: store the reserved stack slots as constants somewhere (??)

mod binop;
mod call;
mod class;
mod cmp;
mod field;
mod import;
mod index;
mod string;
mod truth;

use std::mem::take;

use indexmap::IndexSet;

use super::op;
use crate::ctx::Context;
use crate::error::Error;
use crate::util::JoinIter;
use crate::value::handle::Handle;
use crate::value::object::frame::{Frame, Stack};
use crate::value::object::module::{ModuleId, ModuleLoader, ModuleRegistry};
use crate::value::object::{frame, Class, ClassSuperProxy, Dict, Function, List, ObjectType, Str};
use crate::value::Value;

// in progress:
// - instead of `&[Value]` as args, pass around something like `Args<'a>` that
//   also contains kwargs
// - `Args<'a>` will support:
//   - holding a reference to the stack directly by using a raw const pointer
//   - holding a receiver (as `Option`)
// and then finally de-duplicate the call machinery!!!

// TODO: fields should be private, even to ops
pub struct Isolate {
  ctx: Context,
  globals: Handle<Dict>,
  module_registry: ModuleRegistry,
  module_loader: Box<dyn ModuleLoader>,
  module_init_visited: IndexSet<ModuleId>,

  width: op::Width,
  pc: usize,

  acc: Value,
  frames: Vec<Frame>,
  stdout: Box<dyn Stdout>,
}

pub trait Stdout: std::io::Write + std::any::Any {
  fn as_any(&self) -> &dyn std::any::Any;
}
impl<T: std::io::Write + std::any::Any> Stdout for T {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
}

impl Isolate {
  pub fn new(
    ctx: Context,
    stdout: Box<dyn Stdout>,
    module_loader: Box<dyn ModuleLoader>,
  ) -> Isolate {
    let globals = ctx.alloc(Dict::new());
    Isolate {
      ctx,
      globals,
      module_registry: ModuleRegistry::new(),
      module_loader,
      module_init_visited: IndexSet::new(),
      width: op::Width::Single,
      pc: 0,

      acc: Value::none(),
      frames: vec![],
      stdout,
    }
  }

  pub fn alloc<T: ObjectType>(&self, v: T) -> Handle<T> {
    self.ctx.alloc(v)
  }

  pub fn io(&self) -> &dyn Stdout {
    &*self.stdout
  }

  pub fn print(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
    self.stdout.write_fmt(args)
  }

  pub fn ctx(&self) -> Context {
    self.ctx.clone()
  }

  fn push_frame(&mut self, frame: Frame) {
    self.frames.push(frame);
  }

  fn pop_frame(&mut self) -> Frame {
    self.frames.pop().expect("call stack underflow")
  }

  fn current_frame(&self) -> &Frame {
    self.frames.last().unwrap()
  }

  fn current_frame_mut(&mut self) -> &mut Frame {
    self.frames.last_mut().unwrap()
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
    frame.captures[slot as usize].clone()
  }

  fn set_capture(&mut self, slot: u32, value: Value) {
    let frame = self.current_frame_mut();
    frame.captures[slot as usize] = value;
  }

  fn get_module_var(&self, slot: u32) -> Value {
    let frame = self.current_frame();
    let module_vars = frame.module_vars.as_ref().unwrap();
    let var = module_vars.get_index(slot as usize).unwrap().1;
    var.clone()
  }

  fn set_module_var(&mut self, slot: u32, value: Value) {
    let frame = self.current_frame_mut();
    let module_vars = frame.module_vars.as_mut().unwrap();
    let var = module_vars.get_index_mut(slot as usize).unwrap().1;
    *var = value;
  }

  pub fn get_global(&self, name: &str) -> Option<Value> {
    self.globals.get(name).cloned()
  }

  pub fn set_global(&mut self, name: Handle<Str>, value: Value) {
    self.globals.insert(name, value);
  }

  fn current_module_id(&self) -> Option<ModuleId> {
    self.current_frame().module_id
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

impl op::Handler for Isolate {
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

  fn op_load_module_var(&mut self, slot: u32) -> Result<(), Self::Error> {
    self.acc = self.get_module_var(slot);

    Ok(())
  }

  fn op_store_module_var(&mut self, slot: u32) -> Result<(), Self::Error> {
    self.set_module_var(slot, self.acc.clone());

    Ok(())
  }

  fn op_load_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // global name is always a string
    let name = name.to_str().unwrap();
    let name = name.as_str();
    match self.globals.get(name) {
      Some(v) => self.acc = v.clone(),
      None => return Err(Error::runtime(format!("undefined global {name}")).into()),
    }

    Ok(())
  }

  fn op_store_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // global name is always a string
    let name = name.to_str().unwrap();
    self.globals.insert(name, self.acc.clone());

    Ok(())
  }

  fn op_load_field(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = name.to_str().unwrap();

    self.acc = field::get(&self.ctx, &self.acc, name)?;

    Ok(())
  }

  fn op_load_field_opt(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = name.to_str().unwrap();

    self.acc = field::get_opt(&self.ctx, &self.acc, name)?;

    Ok(())
  }

  fn op_store_field(&mut self, name: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = name.to_str().unwrap();

    let mut obj = self.get_reg(obj);

    field::set(&self.ctx, &mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_load_index(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    self.acc = index::get(&self.ctx, self.acc.clone(), name)?;

    Ok(())
  }

  fn op_load_index_opt(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    self.acc = index::get_opt(&self.ctx, self.acc.clone(), name)?;

    Ok(())
  }

  fn op_store_index(&mut self, key: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let obj = self.get_reg(obj);
    index::set(&self.ctx, obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_import(&mut self, path: u32, dest: u32) -> Result<(), Self::Error> {
    let path = self.get_const(path);
    // should always be a path
    let path = path.to_path().unwrap();

    let module = import::load(self, path)?;

    self.set_reg(dest, module.into());

    Ok(())
  }

  fn op_import_named(&mut self, path: u32, name: u32, dest: u32) -> Result<(), Self::Error> {
    let path = self.get_const(path);
    let name = self.get_const(name);
    // should always be a path
    let path = path.to_path().unwrap();
    // should always be a string
    let name = name.to_str().unwrap();

    let module = import::load(self, path.clone())?;

    let symbol = match module.module_vars().get(name.as_str()).cloned() {
      Some(symbol) => symbol,
      None => {
        return Err(
          Error::runtime(format!(
            "failed to import `{}` from module `{}`",
            name.as_str(),
            path.segments().iter().join('.')
          ))
          .into(),
        )
      }
    };

    self.set_reg(dest, symbol);

    Ok(())
  }

  fn op_load_self(&mut self) -> Result<(), Self::Error> {
    // receiver is always placed at the base of the current call frame's stack
    let this = self.get_reg(3);

    if let Some(proxy) = this.clone().to_class_super_proxy() {
      self.acc = proxy.class().into();
      return Ok(());
    }

    assert!(this.is_class_instance());
    self.acc = this;

    Ok(())
  }

  fn op_load_super(&mut self) -> Result<(), Self::Error> {
    // receiver is always placed at the base of the current call frame's stack
    let this = self.get_reg(3);

    // all parent class `unwrap()`s here should never panic,
    // because parser checks for parent class

    if let Some(proxy) = this.clone().to_class_super_proxy() {
      // we're in a super class
      // proxy to the next super class in the chain
      self.acc = self
        .alloc(ClassSuperProxy::new(
          proxy.class(),
          //    current  next
          //    |        |
          proxy.parent().parent().unwrap(),
        ))
        .into();
      return Ok(());
    }

    // we're not in a super class yet,
    // proxy to the first super class in the chain
    let Some(this) = this.to_class_instance() else {
      // TODO: span
      return Err(Error::runtime("receiver is not a class").into());
    };
    let parent = this.parent().unwrap();
    self.acc = self.ctx.alloc(ClassSuperProxy::new(this, parent)).into();

    Ok(())
  }

  fn op_push_none(&mut self) -> Result<(), Self::Error> {
    self.acc = Value::none();

    Ok(())
  }

  fn op_push_true(&mut self) -> Result<(), Self::Error> {
    self.acc = true.into();

    Ok(())
  }

  fn op_push_false(&mut self) -> Result<(), Self::Error> {
    self.acc = false.into();

    Ok(())
  }

  fn op_push_small_int(&mut self, value: i32) -> Result<(), Self::Error> {
    self.acc = value.into();

    Ok(())
  }

  fn op_create_empty_list(&mut self) -> Result<(), Self::Error> {
    self.acc = self.ctx.alloc(List::new()).into();

    Ok(())
  }

  fn op_push_to_list(&mut self, list: u32) -> Result<(), Self::Error> {
    let list = self.get_reg(list);

    let Some(mut list) = list.to_list() else {
      // TODO: span
      return Err(Error::runtime("value is not a list").into());
    };

    list.push(take(&mut self.acc));

    Ok(())
  }

  fn op_create_empty_dict(&mut self) -> Result<(), Self::Error> {
    self.acc = self.ctx.alloc(Dict::new()).into();

    Ok(())
  }

  fn op_insert_to_dict(&mut self, key: u32, dict: u32) -> Result<(), Self::Error> {
    let key = self.get_reg(key);
    let dict = self.get_reg(dict);
    let key = key.to_str().unwrap();
    let mut dict = dict.to_dict().unwrap();

    // `name` is a `Key` so this `unwrap` won't panic
    dict.insert(key, take(&mut self.acc));

    Ok(())
  }

  fn op_insert_to_dict_named(&mut self, name: u32, dict: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    let dict = self.get_reg(dict);
    let name = name.to_str().unwrap();
    let mut dict = dict.to_dict().unwrap();

    // name used in named load is always a string
    dict.insert(name, take(&mut self.acc));

    Ok(())
  }

  fn op_create_function(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);

    // this should always be a function descriptor
    let desc = desc.to_function_descriptor().unwrap();

    // TODO: module_index
    self.acc = Function::new(&self.ctx, desc, self.current_module_id()).into();

    Ok(())
  }

  fn op_capture_reg(&mut self, reg: u32, slot: u32) -> Result<(), Self::Error> {
    let value = self.get_reg(reg);

    // this should always be a function
    let func = self.acc.clone().to_function().unwrap();

    func.captures()[slot as usize] = value;

    Ok(())
  }

  fn op_capture_slot(&mut self, parent_slot: u32, self_slot: u32) -> Result<(), Self::Error> {
    let value = self.get_capture(parent_slot);

    // this should always be a function
    let func = self.acc.clone().to_function().unwrap();

    func.captures()[self_slot as usize] = value;

    Ok(())
  }

  fn op_create_class_empty(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = desc.to_class_descriptor().unwrap();

    self.acc = Class::new(&self.ctx, desc, &[])?.into();

    Ok(())
  }

  fn op_create_class(&mut self, desc: u32, start: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = desc.to_class_descriptor().unwrap();

    self.acc = Class::new(&self.ctx, desc, &self.stack()[start as usize..])?.into();

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
      return Err(Error::runtime("value is not a bool").into());
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
      (-value).into()
    } else if let Some(value) = value.to_float() {
      (-value).into()
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

    self.acc = value.into();

    Ok(())
  }

  fn op_cmp_eq(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = matches!(ord, Some(cmp::Ordering::Equal)).into();

    Ok(())
  }

  fn op_cmp_neq(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = (!matches!(ord, Some(cmp::Ordering::Equal))).into();

    Ok(())
  }

  fn op_cmp_gt(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = (matches!(ord, Some(cmp::Ordering::Greater))).into();

    Ok(())
  }

  fn op_cmp_ge(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = (matches!(ord, Some(cmp::Ordering::Greater | cmp::Ordering::Equal))).into();

    Ok(())
  }

  fn op_cmp_lt(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = (matches!(ord, Some(cmp::Ordering::Less))).into();

    Ok(())
  }

  fn op_cmp_le(&mut self, lhs: u32) -> Result<(), Self::Error> {
    // TODO: object overload
    let lhs = self.get_reg(lhs);
    let rhs = take(&mut self.acc);

    let ord = cmp::partial_cmp(lhs, rhs)?;

    self.acc = (matches!(ord, Some(cmp::Ordering::Equal | cmp::Ordering::Less))).into();

    Ok(())
  }

  fn op_is_none(&mut self) -> Result<(), Self::Error> {
    self.acc = self.acc.is_none().into();

    Ok(())
  }

  fn op_print(&mut self) -> Result<(), Self::Error> {
    let value = take(&mut self.acc);
    self
      .print(format_args!("{}\n", string::stringify(value)))
      // TODO: span
      .map_err(|_| Error::runtime("failed to print value"))?;
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
          .map_err(|_| Error::runtime("failed to print values"))?;
      } else {
        self
          .print(format_args!("{}", string::stringify(value.clone())))
          // TODO: span
          .map_err(|_| Error::runtime("failed to print values"))?;
      }
    }
    self
      .print(format_args!("\n"))
      // TODO: span
      .map_err(|_| Error::runtime("failed to print values"))?;

    Ok(())
  }

  // TODO: deduplicate call handlers

  fn op_call0(&mut self, return_address: usize) -> Result<(), Self::Error> {
    let callable = self.acc.clone();

    self.call(callable, &[], Value::none(), Some(return_address))?;

    Ok(())
  }

  fn op_call(&mut self, start: u32, args: u32, return_address: usize) -> Result<(), Self::Error> {
    let callable = self.acc.clone();
    // TODO: remove `to_vec` somehow
    let args = self.stack()[start as usize..][..args as usize].to_vec();

    self.call(callable, &args, Value::none(), Some(return_address))?;

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

    self.call(callable, &args, kwargs, Some(return_address))?;

    Ok(())
  }

  fn op_is_pos_param_not_set(&mut self, index: u32) -> Result<(), Self::Error> {
    let frame = self.current_frame();
    let index = index as usize;

    self.acc = (frame.num_args <= index).into();

    Ok(())
  }

  fn op_is_kw_param_not_set(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // base + 2 is always the kw dictionary
    let kwargs = self.get_reg(2);
    let name = name.to_str().unwrap();
    let name = name.as_str();
    let kwargs = kwargs.to_dict().unwrap();

    self.acc = (!kwargs.contains_key(name)).into();

    Ok(())
  }

  fn op_load_kw_param(&mut self, name: u32, param: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // base + 2 is always the kw dictionary
    let kwargs = self.get_reg(2);
    let name = name.to_str().unwrap();
    let name = name.as_str();
    let mut kwargs = kwargs.to_dict().unwrap();

    self.set_reg(param, kwargs.remove(name).unwrap());

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
