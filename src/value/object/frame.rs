use std::fmt::Display;
use std::ops::{Index, IndexMut};
use std::ptr::NonNull;
use std::slice::SliceIndex;

use super::func::func_name;
use super::module::{ModuleId, ModuleRegistry};
use super::{Access, Dict, List};
use crate::ctx::Context;
use crate::value::constant::Constant;
use crate::value::handle::Handle;
use crate::value::Value;
use crate::RuntimeError;

pub struct Frame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  #[allow(dead_code)]
  func: Value,
  pub code: NonNull<[u8]>,
  pub const_pool: NonNull<[Constant]>,
  pub frame_size: usize,
  pub captures: NonNull<[Value]>,
  pub module_vars: Option<NonNull<Dict>>,
  pub module_id: Option<ModuleId>,

  pub on_return: OnReturn,
  pub stack: Stack,
  pub num_args: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum OnReturn {
  Jump(usize),
  Yield,
}

impl Frame {
  /// Create a new call frame.
  ///
  /// # Panics
  ///
  /// If `func` is not a function, closure, or method.
  pub fn new(
    ctx: Context,
    modules: &ModuleRegistry,
    func: Value,
    num_args: usize,
    on_return: OnReturn,
  ) -> Result<Frame, RuntimeError> {
    Self::with_stack(
      modules,
      func,
      num_args,
      on_return,
      Stack::with_capacity(ctx, 256),
    )
  }

  pub fn with_stack(
    modules: &ModuleRegistry,
    func: Value,
    num_args: usize,
    on_return: OnReturn,
    stack: Stack,
  ) -> Result<Self, RuntimeError> {
    if let Some(f) = func.clone().to_method() {
      return Frame::with_stack(modules, f.func(), num_args, on_return, stack);
    }

    let Parts {
      code,
      const_pool,
      frame_size,
      captures,
      module_vars,
      module_id,
    } = get_parts(modules, func.clone())?;

    let mut stack = stack;
    stack.extend(frame_size);

    Ok(Frame {
      func,
      code,
      const_pool,
      on_return,
      stack,
      captures,
      module_vars,
      module_id,
      frame_size,
      num_args,
    })
  }

  pub fn stack_base(&self) -> usize {
    self.stack.base
  }

  pub fn name(&self) -> String {
    func_name(&self.func)
  }
}

impl Access for Frame {}

struct Parts {
  code: NonNull<[u8]>,
  const_pool: NonNull<[Constant]>,
  frame_size: usize,
  captures: NonNull<[Value]>,
  module_vars: Option<NonNull<Dict>>,
  module_id: Option<ModuleId>,
}

fn get_parts(modules: &ModuleRegistry, callable: Value) -> Result<Parts, RuntimeError> {
  let Some(mut func) = callable.clone().to_function() else {
    panic!("cannot create frame from {callable}");
  };

  let mut desc = func.descriptor();
  let code = NonNull::from(unsafe { desc.code_mut() });
  let const_pool = NonNull::from(desc.const_pool());
  let frame_size = desc.frame_size() as usize;
  let captures = NonNull::from(unsafe { func.captures_mut() });
  let (module_vars, module_id) = match func.module_id() {
    Some(id) => {
      let mut module = modules.by_id(id).ok_or_else(|| {
        RuntimeError::script(
          "attempted to call {callable} which was declared in a broken module",
          0..0,
        )
      })?;
      let module_vars = NonNull::from(unsafe { module.module_vars_mut() });
      (Some(module_vars), Some(id))
    }
    None => (None, None),
  };
  return Ok(Parts {
    code,
    const_pool,
    frame_size,
    captures,
    module_vars,
    module_id,
  });
}

impl Drop for Frame {
  fn drop(&mut self) {
    self.stack.truncate(self.stack_base())
  }
}

impl Display for Frame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<frame>")
  }
}

pub struct Stack {
  inner: Handle<List>,
  base: usize,
}

impl Stack {
  pub fn with_capacity(ctx: Context, capacity: usize) -> Self {
    Self {
      inner: ctx.alloc(List::with_capacity(capacity)),
      base: 0,
    }
  }

  pub fn view(other: &Stack, base: usize) -> Self {
    Self {
      inner: other.inner.clone(),
      base,
    }
  }

  pub fn extend(&mut self, n: usize) {
    self.inner.extend((0..n).map(|_| Value::none()));
  }

  pub fn truncate(&mut self, len: usize) {
    self.inner.truncate(len)
  }

  pub fn base(&self) -> usize {
    self.base
  }
}

impl<Idx> Index<Idx> for Stack
where
  Idx: SliceIndex<[Value]>,
{
  type Output = Idx::Output;

  #[inline(always)]
  fn index(&self, index: Idx) -> &Self::Output {
    self.inner[self.base..].index(index)
  }
}

impl<Idx> IndexMut<Idx> for Stack
where
  Idx: SliceIndex<[Value]>,
{
  #[inline]
  fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
    self.inner[self.base..].index_mut(index)
  }
}
