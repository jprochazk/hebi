use std::ptr::NonNull;

use indexmap::IndexSet;
use value::object::frame::{Frame, Stack};
use value::object::handle::Handle;
use value::object::{frame, func, Dict};
use value::Value;

use crate::util::JoinIter;
use crate::{Control, Error, Isolate};

impl<Io: std::io::Write> Isolate<Io> {
  pub fn call(&mut self, f: Value, args: &[Value], kwargs: Value) -> Result<Value, Error> {
    let frame = self.prepare_call_frame(f, args, kwargs, frame::Return::Yield)?;
    self.frames.push(frame);

    self.pc = 0;
    self.dispatch()?;

    // # Return
    Ok(std::mem::take(&mut self.acc))
  }

  pub(crate) fn prepare_call_frame(
    &mut self,
    f: Value,
    args: &[Value],
    kwargs: Value,
    on_return: frame::Return,
  ) -> Result<Handle<Frame>, Error> {
    // # Check that callee is callable
    // TODO: trait
    if !f.is_func() && !f.is_closure() && !f.is_method() {
      return Err(Error::new("value is not callable"));
    }

    // # Check arguments
    let param_info = check_func_args(f.clone(), args, &kwargs)?;

    let stack = match self.frames.last_mut() {
      Some(frame) => {
        let frame = frame.borrow();
        Stack::view(&frame.stack, frame.stack_base() + frame.frame_size)
      }
      None => Stack::new(),
    };

    // # Create a new call frame
    let mut frame = Frame::with_stack(f.clone(), args.len(), on_return, stack);

    // # Initialize params
    init_params(f, &mut frame.stack, param_info, args, kwargs);

    Ok(frame.into())
  }

  fn dispatch(&mut self) -> Result<(), Error> {
    // SAFETY:
    // - `bc` is a valid pointer because of the invariants of `CallFrame::new`
    //   constructor
    // - `pc` is a valid pointer because it is constructed from a mutable reference,
    //   which always results in a valid non-null pointer.
    let mut bc = self.frames.last().unwrap().borrow().code;
    let pc = NonNull::from(&mut self.pc);
    loop {
      match unsafe { op::run(self, bc, pc) } {
        Ok(()) => return Ok(()),
        Err(e) => match e {
          Control::Error(e) => return Err(e),
          Control::SwapFrame => bc = self.frames.last().unwrap().borrow().code,
          Control::Yield => return Ok(()),
        },
      };
    }
  }
}

// TODO: maybe refactor this to not be as unsightly

// Returns `(has_argv, max_params)`
fn check_func_args(func: Value, args: &[Value], kwargs: &Value) -> Result<ParamInfo, Error> {
  fn check_func_args_inner(
    has_implicit_receiver: bool,
    func: Value,
    args: &[Value],
    kwargs: &Value,
  ) -> Result<ParamInfo, Error> {
    if let Some(f) = func.as_func() {
      if let Some(kw) = kwargs.as_dict() {
        check_args(has_implicit_receiver, f.params(), args, &kw)
      } else {
        check_args(has_implicit_receiver, f.params(), args, &Dict::new())
      }
    } else if let Some(f) = func.as_closure() {
      if let Some(kw) = kwargs.as_dict() {
        check_args(has_implicit_receiver, &f.params(), args, &kw)
      } else {
        check_args(has_implicit_receiver, &f.params(), args, &Dict::new())
      }
    } else {
      panic!("check_func_args not implemented for {func}");
    }
  }
  if let Some(m) = func.as_method() {
    return check_func_args_inner(true, m.func.clone(), args, kwargs);
  }
  check_func_args_inner(false, func, args, kwargs)
}

pub fn check_args(
  has_implicit_receiver: bool,
  params: &func::Params,
  args: &[Value],
  kw: &Dict,
) -> Result<ParamInfo, Error> {
  let has_self_param = params.has_self && !has_implicit_receiver;

  let (min, max) = (
    has_self_param as usize + params.min,
    has_self_param as usize + params.max,
  );

  let out_info = ParamInfo {
    has_kw: params.kwargs || !params.kw.is_empty(),
    has_argv: params.argv,
    max_params: max,
  };

  // check positional arguments
  if args.len() < min {
    return Err(Error::new(format!(
      "missing required positional params: {}",
      if has_self_param { Some("self") } else { None }
        .into_iter()
        .chain(params.pos[args.len()..min].iter().map(|s| s.as_str()))
        .join(", "),
    )));
  }
  if !params.argv && args.len() > max {
    return Err(Error::new(format!(
      "expected at most {} args, got {}",
      max,
      args.len()
    )));
  }

  // check kw arguments
  let mut unknown = IndexSet::new();
  let mut missing = IndexSet::new();
  for key in kw.iter().flat_map(|(k, _)| k.as_str()) {
    if !params.kwargs && !params.kw.contains_key(&key[..]) {
      unknown.insert(key.to_string());
    }
  }
  for key in params
    .kw
    .iter()
    .filter_map(|(k, v)| if !*v { Some(k.as_str()) } else { None })
  {
    if !kw.contains_key(key) {
      missing.insert(key.to_string());
    }
  }
  if !unknown.is_empty() || !missing.is_empty() {
    return Err(Error::new(format!(
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
  max_params: usize,
}

#[allow(clippy::identity_op, clippy::needless_range_loop)]
fn init_params(f: Value, stack: &mut Stack, param_info: ParamInfo, args: &[Value], kwargs: Value) {
  // function
  stack.set(0, f.clone());

  // argv
  if param_info.has_argv && args.len() > param_info.max_params {
    let argv = args[param_info.max_params..args.len()].to_vec();
    stack.set(1, Value::from(argv));
  } else {
    stack.set(1, Value::from(vec![]));
  }

  // kwargs
  if param_info.has_kw {
    stack.set(
      2,
      if kwargs.is_none() {
        Value::from(Dict::new())
      } else {
        kwargs
      },
    )
  };

  // params
  let mut params_base = 3;
  if let Some(m) = f.as_method() {
    stack.set(params_base, Value::object(m.this.clone().widen()));
    params_base += 1;
  }
  for i in 0..std::cmp::min(args.len(), param_info.max_params) {
    stack.set(params_base + i, args[i].clone());
  }
}
