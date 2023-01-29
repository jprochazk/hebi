use std::ptr::NonNull;

use indexmap::IndexSet;
use value::object::{func, Dict};
use value::Value;

use crate::util::JoinIter;
use crate::{Error, Isolate};

impl Isolate {
  pub fn call(&mut self, f: Value, args: &[Value], kw: Dict) -> Result<Value, Error> {
    // # Check that callee is callable
    if !f.is_func() && !f.is_closure() {
      return Err(Error::new("value is not callable"));
    }

    // # Check arguments
    let params = check_args(f.clone(), args, &kw)?;

    let parent_pc = self.pc;
    // TODO: minimum 4 stack slots
    // this doesn't work because we're using frame_size which may be 0 (for <main>)
    let stack_base = match self.call_stack.last() {
      Some(frame) => frame.stack_base + frame.frame_size,
      // at minimum, we need 4 stack slots
      None => 4,
    };

    // # Create a new call frame
    let frame = CallFrame::new(f.clone(), parent_pc, stack_base);
    self
      .stack
      .extend((0..frame.frame_size).map(|_| Value::none()));
    self.call_stack.push(frame);

    // # Initialize params
    init_params(f, &mut self.stack, stack_base, params, args, kw);

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

fn check_args(f: Value, args: &[Value], kw: &Dict) -> Result<func::Params, Error> {
  let params = if let Some(f) = f.as_func() {
    f.params().clone()
  } else if let Some(f) = f.as_closure() {
    f.params().clone()
  } else {
    unreachable!()
  };
  if args.len() < params.min {
    return Err(Error::new(format!(
      "missing required positional params: {}",
      params.pos[params.min - args.len()..].iter().join(", "),
    )));
  }
  if !params.argv && args.len() > params.max {
    return Err(Error::new(format!(
      "expected at most {} args, got {}",
      params.max,
      args.len()
    )));
  }
  if params.kw.is_empty() && !kw.is_empty() {
    return Err(Error::new(format!(
      "missing required keyword params: {}",
      params.kw.iter().join(", "),
    )));
  }
  let mut unknown = IndexSet::new();
  let mut matched_kw = 0;
  for key in kw.iter().flat_map(|(k, _)| k.as_str()) {
    if !params.kw.contains(&key[..]) {
      unknown.insert(key.to_string());
    } else {
      matched_kw += 1;
    }
  }
  if !unknown.is_empty() {
    return Err(Error::new(format!(
      "unknown keyword params: {}",
      unknown.iter().join(", ")
    )));
  }
  if matched_kw != params.kw.len() {
    let mut missing = IndexSet::new();
    for key in kw.iter().flat_map(|(k, _)| k.as_str()) {
      if !params.kw.contains(&key[..]) {
        missing.insert(key.to_string());
      }
    }
    return Err(Error::new(format!(
      "missing required keyword params: {}",
      params.kw.iter().join(", ")
    )));
  }
  Ok(params)
}

#[allow(clippy::identity_op)]
fn init_params(
  f: Value,
  stack: &mut [Value],
  stack_base: usize,
  params: func::Params,
  args: &[Value],
  kw: Dict,
) {
  // receiver + function
  stack[stack_base + 0] = Value::none();
  stack[stack_base + 1] = f;
  // argv
  if params.argv && args.len() > params.max {
    let argv = args[params.max..args.len()].to_vec();
    stack[stack_base + 2] = Value::from(argv);
  } else {
    stack[stack_base + 2] = Value::none();
  }
  // kwargs
  stack[stack_base + 3] = Value::from(kw);
  // params
  for i in 0..std::cmp::min(args.len(), params.max) {
    stack[stack_base + 4 + i] = args[i].clone();
  }
  if args.len() < params.max {
    for i in args.len()..params.max {
      stack[stack_base + 4 + i] = Value::none();
    }
  }
}

pub(crate) struct CallFrame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  #[allow(dead_code)]
  func: Value,
  pub(crate) code: NonNull<[u8]>,
  pub(crate) const_pool: NonNull<[Value]>,
  pub(crate) captures: Option<NonNull<[Value]>>,
  pub(crate) parent_pc: usize,
  pub(crate) stack_base: usize,
  pub(crate) frame_size: usize,
}

impl CallFrame {
  /// Create a new call frame.
  ///
  /// # Panics
  ///
  /// If `func` is not a function or closure.
  pub(crate) fn new(mut func: Value, parent_pc: usize, stack_base: usize) -> Self {
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
      };
    }

    panic!("attempted to create call frame from something that is not a function: {func:?}");
  }
}
