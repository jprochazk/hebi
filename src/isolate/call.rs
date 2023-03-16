use indexmap::IndexSet;

use super::{Control, Isolate};
use crate::ctx::Context;
use crate::util::JoinIter;
use crate::value::handle::Handle;
use crate::value::object::frame::{Frame, Stack, StackSlice};
use crate::value::object::func::Function;
use crate::value::object::{frame, func, Dict, List};
use crate::value::Value;
use crate::{op, Error, Result};

#[derive(Clone)]
pub struct Args {
  this: Value,
  args: Option<StackSlice>,
  kwargs: Option<Handle<Dict>>,
}

impl Args {
  pub fn new(this: Value, args: Option<StackSlice>, kwargs: Option<Handle<Dict>>) -> Self {
    Self { this, args, kwargs }
  }

  pub fn with_receiver(self, this: Value) -> Self {
    Self {
      this,
      args: self.args,
      kwargs: self.kwargs,
    }
  }

  pub fn empty() -> Self {
    Self {
      this: Value::none(),
      args: None,
      kwargs: None,
    }
  }

  pub fn this(&self) -> &Value {
    &self.this
  }

  pub fn pos(&self, index: usize) -> Option<Value> {
    match &self.args {
      Some(slice) => Some(slice[index].clone()),
      None => panic!("{index} out of bounds in stack slice of len 0"),
    }
  }

  pub fn num_positional(&self) -> usize {
    match &self.args {
      Some(slice) => slice.len(),
      None => 0,
    }
  }

  pub fn kw(&self, key: &str) -> Option<Value> {
    self
      .kwargs
      .as_ref()
      .and_then(|kwargs| kwargs.get(key).cloned())
  }

  pub fn has_kw(&self) -> bool {
    self.kwargs.is_some()
  }

  pub fn keys(&self) -> impl Iterator<Item = &str> {
    self
      .kwargs
      .iter()
      .flat_map(|kwargs| kwargs.keys().map(|k| k.as_str()))
  }

  pub(crate) fn all_kw(&self) -> Option<Handle<Dict>> {
    self.kwargs.clone()
  }

  pub(crate) unsafe fn all_pos(&self) -> &[Value] {
    match &self.args {
      Some(slice) => &slice[..],
      None => &[],
    }
  }
}

// TODO: check for stack overflow in `run` and `call_recurse`

impl Isolate {
  /// Run `func` to completion or until a yield point.
  ///
  /// This is the entry point for bytecode execution.
  pub fn run(&mut self, func: Handle<Function>) -> Result<Value> {
    let frame = self.prepare_call(func, Args::empty(), frame::OnReturn::Yield)?;
    let frame_depth = self.frames.len();
    // this frame will be popped by the `ret` instruction at the end of `func`
    self.push_frame(frame);

    self.width = op::Width::Single;
    let saved_pc = self.pc;
    self.pc = 0;
    if let Err(e) = self.run_dispatch_loop() {
      for _ in frame_depth..self.frames.len() {
        self.pop_frame();
      }
      return Err(e);
    }
    self.pc = saved_pc;

    // # Return
    Ok(std::mem::take(&mut self.acc))
  }

  /// Run `func` to completion or until a yield point.
  pub fn call_recurse(&mut self, func: Handle<Function>, args: Args) -> Result<Value> {
    let frame = self.prepare_call(func, args, frame::OnReturn::Yield)?;
    let frame_depth = self.frames.len();
    // this frame will be popped by the `ret` instruction at the end of `func`
    self.push_frame(frame);

    self.width = op::Width::Single;
    let saved_pc = self.pc;
    self.pc = 0;
    if let Err(e) = self.run_dispatch_loop() {
      for _ in frame_depth..self.frames.len() {
        self.pop_frame();
      }
      return Err(e);
    }
    self.pc = saved_pc;

    // # Return
    Ok(std::mem::take(&mut self.acc))
  }

  /// Call `callable` with the given `args`.
  pub fn call(&mut self, callable: Value, args: Args, return_address: Option<usize>) -> Result<()> {
    let return_address = return_address.unwrap_or(self.pc);
    if callable.is_class() {
      // class constructor
      let def = callable.to_class().unwrap();
      self.acc = self.create_instance(def, args)?;
      self.pc = return_address;
      return Ok(());
    }

    if callable.is_native_class() {
      let class = callable.to_native_class().unwrap();
      self.acc = self.create_native_instance(class, args)?;
      self.pc = return_address;
      return Ok(());
    }

    if let Some(f) = callable.clone().to_native_function() {
      self.acc = f.call(&self.ctx, args)?;
      self.pc = return_address;
      return Ok(());
    }

    if let Some(m) = callable.clone().to_method() {
      if let Some(f) = m.func().to_native_function() {
        let this = match m.this().to_native_class_instance() {
          Some(instance) => instance.user_data().into(),
          None => m.this(),
        };
        self.acc = f.call(&self.ctx, args.with_receiver(this))?;
        self.pc = return_address;
        return Ok(());
      }
    }

    let (callable, args) = if let Some(m) = callable.clone().to_method() {
      (m.func(), args.with_receiver(m.this()))
    } else {
      (callable, args)
    };
    let Some(callable) = callable.clone().to_function() else {
      return Err(Error::runtime(format!("cannot call `{callable}`")));
    };

    // regular function call
    let on_return = frame::OnReturn::Jump(return_address);
    let frame = self.prepare_call(callable, args, on_return)?;
    // this frame will be popped by the `ret` instruction at the end of `func`
    self.push_frame(frame);
    self.width = op::Width::Single;
    self.pc = 0;
    Ok(())
  }

  pub fn prepare_call(
    &mut self,
    callable: Handle<Function>,
    args: Args,
    on_return: frame::OnReturn,
  ) -> Result<Frame> {
    // # Check arguments
    let descriptor = callable.descriptor();
    let param_info = check_args(!args.this().is_none(), descriptor.params(), &args)?;

    // # Create a new call frame
    let mut frame = match self.frames.last_mut() {
      Some(frame) => Frame::with_stack(
        &self.module_registry,
        callable.clone(),
        args.num_positional(),
        on_return,
        Stack::view(&frame.stack, frame.stack_base() + frame.frame_size),
      )?,
      None => Frame::new(
        self.ctx.clone(),
        &self.module_registry,
        callable.clone(),
        args.num_positional(),
        on_return,
      )?,
    };

    // # Initialize params
    init_params(
      self.ctx.clone(),
      callable,
      &mut frame.stack,
      &param_info,
      args,
    );

    Ok(frame)
  }

  fn run_dispatch_loop(&mut self) -> crate::Result<()> {
    let result = loop {
      let code = self.current_frame_mut().code;
      match op::dispatch(self, code, self.pc, self.width) {
        Ok(flow) => match flow {
          (op::ControlFlow::Jump(offset), w) => {
            self.width = w;
            self.pc += offset;
          }
          (op::ControlFlow::Loop(offset), w) => {
            self.width = w;
            self.pc -= offset;
          }
          (op::ControlFlow::Yield, _) => break Ok(()),
          (op::ControlFlow::Nop, w) => {
            self.width = w;
          }
        },
        Err(e) => match e {
          Control::Error(e) => break Err(e),
          Control::Yield => break Ok(()),
        },
      };
    };
    result
  }
}

pub fn check_args(
  has_implicit_receiver: bool,
  params: &func::Params,
  args: &Args,
) -> crate::Result<ParamInfo> {
  let has_self_param = params.has_self && !has_implicit_receiver;

  let (min, max) = (
    has_self_param as usize + params.min,
    has_self_param as usize + params.max,
  );

  let out_info = ParamInfo {
    has_kw: params.kwargs || !params.kw.is_empty(),
    has_argv: params.argv,
    has_self: params.has_self,
    max_params: max,
  };

  // check positional arguments
  if args.num_positional() < min {
    return Err(Error::runtime(format!(
      "missing required positional params: {}",
      if has_self_param { Some("self") } else { None }
        .into_iter()
        .chain(
          params.pos[args.num_positional()..min]
            .iter()
            .map(|s| s.as_str())
        )
        .join(", "),
    )));
  }
  if !params.argv && args.num_positional() > max {
    return Err(Error::runtime(format!(
      "expected at most {} args, got {}",
      max,
      args.num_positional()
    )));
  }

  // TODO: deduplicate with `util`
  // check kw arguments
  let mut unknown = IndexSet::new();
  let mut missing = IndexSet::new();
  if args.has_kw() {
    // we have kwargs,
    // - check for unknown keywords
    for key in args.keys() {
      if !params.kwargs && !params.kw.contains_key(key) {
        unknown.insert(key.to_string());
      }
    }
    // - check for missing keywords
    for key in params
      .kw
      .iter()
      // only check required keyword params
      .filter_map(|(k, v)| if !*v { Some(k.as_str()) } else { None })
    {
      if args.kw(key).is_none() {
        missing.insert(key.to_string());
      }
    }
  } else {
    // we don't have kwargs,
    // just check for missing keyword params
    missing.extend(params.kw.iter().filter_map(|(k, v)| {
      // only check required keyword params
      if !*v {
        Some(k.as_str().to_string())
      } else {
        None
      }
    }))
  }
  // if we have a mismatch, output a comprehensive error
  if !unknown.is_empty() || !missing.is_empty() {
    return Err(Error::runtime(format!(
      "mismatched keyword params: {}{}{}",
      if !unknown.is_empty() {
        format!("could not recognize {}", unknown.iter().join(", "))
      } else {
        String::new()
      },
      if !unknown.is_empty() && !missing.is_empty() {
        " and "
      } else {
        ""
      },
      if !missing.is_empty() {
        format!("missing {}", missing.iter().join(", "))
      } else {
        String::new()
      },
    )));
  }

  Ok(out_info)
}

pub struct ParamInfo {
  has_kw: bool,
  has_argv: bool,
  has_self: bool,
  max_params: usize,
}

#[allow(clippy::identity_op, clippy::needless_range_loop)]
fn init_params(
  ctx: Context,
  func: Handle<Function>,
  stack: &mut Stack,
  param_info: &ParamInfo,
  args: Args,
) {
  stack[0] = func.into();

  if args.num_positional() > param_info.max_params {
    let mut argv = Vec::with_capacity(args.num_positional() - param_info.max_params);
    for i in param_info.max_params..args.num_positional() {
      argv.push(args.pos(i).unwrap())
    }
    stack[1] = ctx.alloc(List::from(argv)).into();
  } else if param_info.has_argv {
    stack[1] = ctx.alloc(List::new()).into();
  } else {
    stack[1] = Value::none();
  }

  if !args.has_kw() && param_info.has_kw {
    stack[2] = ctx.alloc(Dict::new()).into();
  } else {
    stack[2] = args.all_kw().map(Value::from).unwrap_or(Value::none());
  }

  let len = std::cmp::min(args.num_positional(), param_info.max_params);
  if args.this().is_none() && param_info.has_self {
    stack[3] = args.pos(0).unwrap();
    for i in 1..len {
      stack[4 + i] = args.pos(i).unwrap();
    }
  } else {
    stack[3] = args.this().clone();
    for i in 0..len {
      stack[4 + i] = args.pos(i).unwrap();
    }
  }
}
