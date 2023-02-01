use std::ptr::NonNull;

use indexmap::IndexSet;
use value::object::{func, Dict};
use value::Value;

use crate::util::JoinIter;
use crate::{Error, Isolate};

impl<Io: std::io::Write> Isolate<Io> {
  pub fn call(&mut self, f: Value, args: &[Value], kw: Value) -> Result<Value, Error> {
    // # Check that callee is callable
    if !f.is_func() && !f.is_closure() {
      return Err(Error::new("value is not callable"));
    }

    // # Check arguments
    let param_info = check_args(f.clone(), args, &kw)?;

    let parent_pc = self.pc;
    let stack_base = match self.call_stack.last() {
      Some(frame) => frame.stack_base + frame.frame_size,
      None => 0,
    };

    // # Create a new call frame
    let frame = CallFrame::new(f.clone(), parent_pc, stack_base, args.len());
    self
      .stack
      .extend((0..frame.frame_size).map(|_| Value::none()));
    self.call_stack.push(frame);

    // # Initialize params
    init_params(f, &mut self.stack, stack_base, param_info, args, kw);

    // # Dispatch
    self.pc = 0;
    let bc = self.call_stack.last_mut().unwrap().code;
    let pc = std::ptr::NonNull::from(&mut self.pc);
    // SAFETY:
    // - `bc` is a valid pointer because of the invariants of `CallFrame::new`
    //   constructor
    // - `pc` is a valid pointer because it is constructed from a mutable reference,
    //   which always results in a valid non-null pointer.
    unsafe { op::run(self, bc, pc)? }

    // # Pop call frame and truncate stack
    let frame = self.call_stack.pop().unwrap();
    self.pc = frame.parent_pc;
    self.stack.truncate(stack_base);

    Ok(std::mem::take(&mut self.acc))
  }
}

// Returns `(has_argv, max_params)`
fn check_args(func: Value, args: &[Value], kw: &Value) -> Result<ParamInfo, Error> {
  fn check_args_inner(
    params: &func::Params,
    args: &[Value],
    kw: &Dict,
  ) -> Result<ParamInfo, Error> {
    let out_info = ParamInfo {
      has_kw: !params.kw.is_empty(),
      has_argv: params.argv,
      max_params: params.max,
    };

    // check positional arguments
    if args.len() < params.min {
      return Err(Error::new(format!(
        "missing required positional params: {}",
        params.pos[args.len()..params.min].iter().join(", "),
      )));
    }
    if !params.argv && args.len() > params.max {
      return Err(Error::new(format!(
        "expected at most {} args, got {}",
        params.max,
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

  if let Some(f) = func.as_func() {
    if let Some(kw) = kw.as_dict() {
      check_args_inner(f.params(), args, &kw)
    } else {
      check_args_inner(f.params(), args, &Dict::new())
    }
  } else if let Some(f) = func.as_closure() {
    if let Some(kw) = kw.as_dict() {
      check_args_inner(&f.params(), args, &kw)
    } else {
      check_args_inner(&f.params(), args, &Dict::new())
    }
  } else {
    unreachable!()
  }
}

struct ParamInfo {
  has_kw: bool,
  has_argv: bool,
  max_params: usize,
}

#[allow(clippy::identity_op)]
fn init_params(
  f: Value,
  stack: &mut [Value],
  stack_base: usize,
  param_info: ParamInfo,
  args: &[Value],
  kw: Value,
) {
  // TODO: init receiver
  // receiver + function
  stack[stack_base + 0] = Value::none();
  stack[stack_base + 1] = f;
  // argv
  if param_info.has_argv && args.len() > param_info.max_params {
    let argv = args[param_info.max_params..args.len()].to_vec();
    stack[stack_base + 2] = Value::from(argv);
  } else {
    stack[stack_base + 2] = Value::from(vec![]);
  }
  // kwargs
  if param_info.has_kw {
    stack[stack_base + 3] = if kw.is_none() {
      Value::from(Dict::new())
    } else {
      kw
    }
  };
  // params
  for i in 0..std::cmp::min(args.len(), param_info.max_params) {
    stack[stack_base + 4 + i] = args[i].clone();
  }
}

pub(crate) struct CallFrame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  #[allow(dead_code)]
  func: Value,
  pub code: NonNull<[u8]>,
  pub const_pool: NonNull<[Value]>,
  pub captures: Option<NonNull<[Value]>>,
  pub parent_pc: usize,
  pub stack_base: usize,
  pub frame_size: usize,
  pub num_args: usize,
}

impl CallFrame {
  /// Create a new call frame.
  ///
  /// # Panics
  ///
  /// If `func` is not a function or closure.
  pub fn new(mut func: Value, parent_pc: usize, stack_base: usize, num_args: usize) -> Self {
    let value = func.clone();
    if let Some(mut f) = func.as_func_mut() {
      let code = NonNull::from(f.code_mut());
      let const_pool = NonNull::from(f.const_pool());
      let frame_size = f.frame_size() as usize;
      return Self {
        func: value,
        code,
        const_pool,
        captures: None,
        parent_pc,
        stack_base,
        frame_size,
        num_args,
      };
    }

    if let Some(mut f) = func.as_closure_mut() {
      let code = NonNull::from(f.code_mut().as_mut());
      let const_pool = NonNull::from(f.const_pool().as_ref());
      let captures = NonNull::from(&mut f.captures[..]);
      let frame_size = f.frame_size() as usize;
      return Self {
        func: value,
        code,
        const_pool,
        captures: Some(captures),
        parent_pc,
        stack_base,
        frame_size,
        num_args,
      };
    }

    panic!("attempted to create call frame from something that is not a function: {func:?}");
  }
}
