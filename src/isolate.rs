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

use super::op;
use crate::ctx::Context;
use crate::value::handle::Handle;
use crate::value::object::frame::{Frame, Stack};
use crate::value::object::module::{ModuleId, ModuleLoader, ModuleRegistry, ModuleSource};
use crate::value::object::{
  frame, ClassDef, Dict, Function, Key, List, Module, ObjectType, Path, Proxy,
};
use crate::value::Value;
use crate::RuntimeError;

// TODO: fields should be private, even to ops
pub struct Isolate {
  ctx: Context,
  globals: Handle<Dict>,
  module_registry: ModuleRegistry,
  module_loader: Box<dyn ModuleLoader>,

  width: op::Width,
  pc: usize,

  acc: Value,
  frames: Vec<Frame>,
  current_frame: Option<NonNull<Frame>>,
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
      width: op::Width::Single,
      pc: 0,

      acc: Value::none(),
      frames: vec![],
      current_frame: None,
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
    let captures = unsafe { frame.captures.as_ref() };
    captures[slot as usize].clone()
  }

  fn set_capture(&mut self, slot: u32, value: Value) {
    let frame = self.current_frame_mut();
    let captures = unsafe { frame.captures.as_mut() };
    captures[slot as usize] = value;
  }

  fn get_module_var(&self, slot: u32) -> Value {
    let frame = self.current_frame();
    let module_vars = unsafe { frame.module_vars.unwrap().as_ref() };
    let var = module_vars.get_index(slot as usize).unwrap().1;
    var.clone()
  }

  fn set_module_var(&mut self, slot: u32, value: Value) {
    let frame = self.current_frame_mut();
    let module_vars = unsafe { frame.module_vars.unwrap().as_mut() };
    let var = module_vars.get_index_mut(slot as usize).unwrap().1;
    *var = value;
  }

  fn current_module_id(&self) -> Option<ModuleId> {
    self.current_frame().module_id
  }

  fn stack(&self) -> &Stack {
    &self.current_frame().stack
  }
}

pub enum Control {
  Error(RuntimeError),
  Yield,
}

impl From<RuntimeError> for Control {
  fn from(value: RuntimeError) -> Self {
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
    self.get_module_var(slot);

    Ok(())
  }

  fn op_store_module_var(&mut self, slot: u32) -> Result<(), Self::Error> {
    self.set_module_var(slot, self.acc.clone());

    Ok(())
  }

  fn op_load_global(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // global name is always a string
    let name = Key::try_from(name).unwrap();
    match self.globals.get(&name) {
      Some(v) => self.acc = v.clone(),
      // TODO: span
      None => return Err(RuntimeError::script(format!("undefined global {name}"), 0..0).into()),
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

    self.acc = field::get(self.ctx.clone(), &self.acc, name)?;

    Ok(())
  }

  fn op_load_field_opt(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = Key::try_from(name).unwrap();
    let name = name.as_str().unwrap();

    self.acc = field::get_opt(self.ctx.clone(), &self.acc, name)?;

    Ok(())
  }

  fn op_store_field(&mut self, name: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name used in named load is always a string
    let name = Key::try_from(name).unwrap();
    let name = name.as_str().unwrap();

    let mut obj = self.get_reg(obj);

    field::set(self.ctx.clone(), &mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_load_index(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = Key::try_from(name.clone()) else {
      // TODO: span
      return Err(RuntimeError::script(format!("{name} is not a valid key"), 0..0).into());
    };

    self.acc = index::get(&self.acc, &name)?;

    Ok(())
  }

  fn op_load_index_opt(&mut self, key: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = Key::try_from(name.clone()) else {
      // TODO: span
      return Err(RuntimeError::script(format!("{name} is not a valid key"), 0..0).into());
    };

    self.acc = index::get_opt(&self.acc, &name)?;

    Ok(())
  }

  fn op_store_index(&mut self, key: u32, obj: u32) -> Result<(), Self::Error> {
    let name = self.get_reg(key);
    let Ok(name) = Key::try_from(name.clone()) else {
      // TODO: span
      return Err(RuntimeError::script(format!("{name} is not a valid key"), 0..0).into());
    };

    let mut obj = self.get_reg(obj);

    index::set(self.ctx.clone(), &mut obj, name, self.acc.clone())?;

    Ok(())
  }

  fn op_import(&mut self, path: u32, dest: u32) -> Result<(), Self::Error> {
    // TODO: move this to its own file

    let path = self.get_const(path);
    // should always be a path
    let path = path.to_path().unwrap();

    let module = self
      .module_loader
      .load(unsafe { path._get() })
      .map_err(|e| RuntimeError::native(e, 0..0))?;

    // TODO: configurable emit for modules that makes them read from globals instead
    // of module_vars, and use it for `eval`.
    match module {
      ModuleSource::Module(source) => {
        let name = path.segments().last().unwrap().as_str();
        let module = crate::emit::emit(self.ctx.clone(), name, source, false).unwrap();
        let module_id = self.module_registry.next_module_id();
        let module = module.instance(&self.ctx, Some(module_id));
        self.module_registry.add(module_id, module.clone());

        // If executing the module root scope results in an error,
        // remove the module from the registry. We do this to ensure
        // that calls to functions declared in this broken module
        // (even in inner scopes) will fail
        let result = self.call(module.root().into(), &[], Value::none());
        if let Err(e) = result {
          self.module_registry.remove(module_id);
          Err(e)?;
        }
      }
      ModuleSource::Native(_) => todo!("native modules are unimplemented"),
    }

    todo!()
  }

  fn op_load_self(&mut self) -> Result<(), Self::Error> {
    // receiver is always placed at the base of the current call frame's stack
    let this = self.get_reg(3);

    if let Some(proxy) = this.clone().to_proxy() {
      self.acc = proxy.class().into();
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
      self.acc = self
        .alloc(Proxy::new(
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
    let Some(this) = this.to_class() else {
      // TODO: span
      return Err(RuntimeError::script("receiver is not a class", 0..0).into());
    };
    let parent = this.parent().unwrap();
    self.acc = self.ctx.alloc(Proxy::new(this, parent)).into();

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
      return Err(RuntimeError::script("value is not a list", 0..0).into());
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
    let Ok(key) = Key::try_from(key.clone()) else {
      // TODO: span
      return Err(RuntimeError::script(format!("{key} is not a valid key"), 0..0).into());
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

  fn op_create_function(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);

    // this should always be a function descriptor
    let desc = desc.to_function_descriptor().unwrap();

    // TODO: module_index
    self.acc = Function::new(&self.ctx, desc, self.current_frame().module_id).into();

    Ok(())
  }

  fn op_capture_reg(&mut self, reg: u32, slot: u32) -> Result<(), Self::Error> {
    let value = self.get_reg(reg);

    // this should always be a function
    let mut func = self.acc.clone().to_function().unwrap();

    let captures = unsafe { func.captures_mut() };
    captures[slot as usize] = value;

    Ok(())
  }

  fn op_capture_slot(&mut self, parent_slot: u32, self_slot: u32) -> Result<(), Self::Error> {
    let value = self.get_capture(parent_slot);

    // this should always be a function
    let mut func = self.acc.clone().to_function().unwrap();

    let captures = unsafe { func.captures_mut() };
    captures[self_slot as usize] = value;

    Ok(())
  }

  fn op_create_class_empty(&mut self, desc: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = desc.to_class_descriptor().unwrap();

    self.acc = self
      .ctx
      .alloc(ClassDef::new(self.ctx.clone(), desc, &[]))
      .into();

    Ok(())
  }

  fn op_create_class(&mut self, desc: u32, start: u32) -> Result<(), Self::Error> {
    let desc = self.get_const(desc);
    // this should always be a class descriptor
    let desc = desc.to_class_descriptor().unwrap();

    let value = self
      .ctx
      .alloc(ClassDef::new(
        self.ctx.clone(),
        desc,
        &self.stack()[start as usize..],
      ))
      .into();
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
      return Err(RuntimeError::script("value is not a bool", 0..0).into());
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
      .map_err(|_| RuntimeError::script("failed to print value", 0..0))?;
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
          .map_err(|_| RuntimeError::script("failed to print values", 0..0))?;
      } else {
        self
          .print(format_args!("{}", string::stringify(value.clone())))
          // TODO: span
          .map_err(|_| RuntimeError::script("failed to print values", 0..0))?;
      }
    }
    self
      .print(format_args!("\n"))
      // TODO: span
      .map_err(|_| RuntimeError::script("failed to print values", 0..0))?;

    Ok(())
  }

  fn op_call0(&mut self, return_address: usize) -> Result<(), Self::Error> {
    let callable = self.acc.clone();

    if callable.is_class_def() {
      // class constructor
      let class_def = callable.to_class_def().unwrap();
      self.acc = self.create_instance(class_def, &[], Value::none())?;
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
      self.acc = self.create_instance(class_def, &args, Value::none())?;
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
      self.acc = self.create_instance(def, &args, kwargs)?;
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

    self.acc = (frame.num_args <= index).into();

    Ok(())
  }

  fn op_is_kw_param_not_set(&mut self, name: u32) -> Result<(), Self::Error> {
    let name = self.get_const(name);
    // name is always a string here
    let name = Key::try_from(name).unwrap();
    // base + 3 is always the kw dictionary
    let kwargs = self.get_reg(2);
    let kwargs = kwargs.to_dict().unwrap();

    self.acc = (!kwargs.contains_key(&name)).into();

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
