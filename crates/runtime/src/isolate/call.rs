use indexmap::IndexSet;

use super::{Control, Isolate};
use crate::util::JoinIter;
use crate::value::object::frame::{Frame, Stack};
use crate::value::object::{frame, func, Dict, Handle, List};
use crate::value::Value;
use crate::{Error, Result};

// TODO: factor `method` out of this process, so `invoke` can be implemented as
// a fast-track, all it needs to do is place the receiver. currently it's
// allocating an extra object.

impl<Io: std::io::Write> Isolate<Io> {
  pub fn call(&mut self, f: Value, args: &[Value], kwargs: Value) -> Result<Value> {
    let frame = self.prepare_call_frame(f, args, kwargs, frame::OnReturn::Yield)?;
    let frame_depth = self.frames.len();
    self.push_frame(frame);

    self.width = op::Width::Single;
    self.pc = 0;
    if let Err(mut e) = self.run_dispatch_loop() {
      for _ in frame_depth..self.frames.len() {
        let frame = self.pop_frame();
        // TODO: Span and file name
        e.push_trace(frame.name(), 0..0, None);
      }
      return Err(e);
    }

    // # Return
    Ok(std::mem::take(&mut self.acc))
  }

  pub(crate) fn prepare_call_frame(
    &mut self,
    callable: Value,
    args: &[Value],
    kwargs: Value,
    on_return: frame::OnReturn,
  ) -> Result<Frame> {
    // # Check that callee is callable
    // TODO: trait
    if !callable.is_func() && !callable.is_closure() && !callable.is_method() {
      // TODO: span
      return Err(Error::new("value is not callable", 0..0));
    }

    // # Check arguments
    let param_info = check_func_args(callable.clone(), args, kwargs.clone())?;

    let stack = match self.frames.last_mut() {
      Some(frame) => Stack::view(&frame.stack, frame.stack_base() + frame.frame_size),
      None => Stack::new(),
    };

    // # Create a new call frame
    let mut frame = Frame::with_stack(callable.clone(), args.len(), on_return, stack);

    // # Initialize params
    init_params(callable, &mut frame.stack, param_info, args, kwargs);

    Ok(frame)
  }

  fn run_dispatch_loop(&mut self) -> Result<(), Error> {
    let result = loop {
      let code = unsafe { self.current_frame_mut().code.as_mut() };
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

// TODO: maybe refactor this to not be as unsightly

// Returns `(has_argv, max_params)`
fn check_func_args(func: Value, args: &[Value], kwargs: Value) -> Result<ParamInfo, Error> {
  fn check_func_args_inner(
    has_implicit_receiver: bool,
    func: Value,
    args: &[Value],
    kwargs: Value,
  ) -> Result<ParamInfo, Error> {
    let kw = kwargs.to_dict();
    if let Some(f) = func.clone().to_func() {
      check_args(has_implicit_receiver, f.params(), args, kw)
    } else if let Some(f) = func.clone().to_closure() {
      check_args(
        has_implicit_receiver,
        f.descriptor().func().params(),
        args,
        kw,
      )
    } else {
      panic!("check_func_args not implemented for {func}");
    }
  }
  if let Some(m) = func.clone().to_method() {
    return check_func_args_inner(true, m.func(), args, kwargs);
  }
  check_func_args_inner(false, func, args, kwargs)
}

pub fn check_args(
  has_implicit_receiver: bool,
  params: &func::Params,
  args: &[Value],
  kw: Option<Handle<Dict>>,
) -> Result<ParamInfo, Error> {
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
  if args.len() < min {
    return Err(Error::new(
      format!(
        "missing required positional params: {}",
        if has_self_param { Some("self") } else { None }
          .into_iter()
          .chain(params.pos[args.len()..min].iter().map(|s| s.as_str()))
          .join(", "),
      ),
      0..0,
    ));
  }
  if !params.argv && args.len() > max {
    return Err(Error::new(
      format!("expected at most {} args, got {}", max, args.len()),
      0..0,
    ));
  }

  // check kw arguments
  let mut unknown = IndexSet::new();
  let mut missing = IndexSet::new();
  if let Some(kw) = kw {
    // we have kwargs,
    // - check for unknown keywords
    for key in kw.iter().flat_map(|(k, _)| k.as_str()) {
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
      if !kw.contains_key(key) {
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
    return Err(Error::new(
      format!(
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
      ),
      0..0,
    ));
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
fn init_params(f: Value, stack: &mut Stack, param_info: ParamInfo, args: &[Value], kwargs: Value) {
  // function
  stack[0] = f.clone();

  // argv
  if param_info.has_argv && args.len() > param_info.max_params {
    let argv = args[param_info.max_params..args.len()].to_vec();
    stack[1] = Value::object(Handle::alloc(List::from(argv)));
  } else {
    stack[1] = Value::object(Handle::alloc(List::new()));
  }

  // kwargs
  if param_info.has_kw {
    stack[2] = if kwargs.is_none() {
      Value::object(Handle::alloc(Dict::new()))
    } else {
      kwargs
    };
  };

  // params
  let mut params_base = 3;
  if let Some(m) = f.to_method() {
    // method call - set implicit receiver
    // because we set it, it can't be part of `args`
    // so we also bump `params_base` to `4`
    stack[params_base] = m.this();
    params_base += 1;
  } else if !param_info.has_self {
    // regular function call without `self`
    // there is no `self` implicitly nor explicitly passed in
    // the first non-self param is at `4`, so we bump `params_base`
    params_base += 1;
  }
  // `args` contains just the params, or in the case of a static call of a method,
  // it will also contain `self`. if it contains `self`, `params_base` must be
  // `3`, because `self` is always at `stack_base + 3`.
  for i in 0..std::cmp::min(args.len(), param_info.max_params) {
    stack[params_base + i] = args[i].clone();
  }
}
