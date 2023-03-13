use std::fmt::Display;
use std::ops::{Index, IndexMut, Range};
use std::ptr::NonNull;
use std::slice::SliceIndex;

use super::module::{ModuleId, ModuleRegistry};
use super::{Access, Dict, Function, List, Str};
use crate::ctx::Context;
use crate::value::constant::Constant;
use crate::value::handle::Handle;
use crate::value::Value;
use crate::{Error, Result};

pub struct Frame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  #[allow(dead_code)]
  func: Handle<Function>,
  pub code: NonNull<[u8]>,
  pub const_pool: NonNull<[Constant]>,
  pub frame_size: usize,
  pub captures: Handle<List>,
  pub module_vars: Option<Handle<Dict>>,
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
    func: Handle<Function>,
    num_args: usize,
    on_return: OnReturn,
  ) -> Result<Frame> {
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
    func: Handle<Function>,
    num_args: usize,
    on_return: OnReturn,
    stack: Stack,
  ) -> Result<Self> {
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

  pub fn name(&self) -> Handle<Str> {
    self.func.descriptor().name()
  }
}

impl Access for Frame {}

struct Parts {
  code: NonNull<[u8]>,
  const_pool: NonNull<[Constant]>,
  frame_size: usize,
  // TODO: update captures same as module_vars
  captures: Handle<List>,
  module_vars: Option<Handle<Dict>>,
  module_id: Option<ModuleId>,
}

fn get_parts(modules: &ModuleRegistry, func: Handle<Function>) -> Result<Parts> {
  let mut desc = func.descriptor();
  // Safety:
  let code = unsafe { desc.code_mut() };
  let const_pool = unsafe { desc.const_pool() };
  let frame_size = desc.frame_size() as usize;
  let captures = func.captures();
  let (module_vars, module_id) = match func.module_id() {
    Some(id) => {
      let module = modules.by_id(id).ok_or_else(|| {
        Error::runtime("attempted to call {callable} which was declared in a broken module")
      })?;
      (Some(module.module_vars()), Some(id))
    }
    None => (None, None),
  };
  Ok(Parts {
    code,
    const_pool,
    frame_size,
    captures,
    module_vars,
    module_id,
  })
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

#[derive(Clone)]
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

  pub fn len(&self) -> usize {
    self.inner[self.base..].len()
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

  pub fn slice(&self, range: Range<usize>) -> StackSlice {
    StackSlice {
      stack: self.clone(),
      range,
    }
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

#[derive(Clone)]
pub struct StackSlice {
  stack: Stack,
  range: Range<usize>,
}

impl StackSlice {
  pub fn len(&self) -> usize {
    self.stack[self.range.clone()].len()
  }
}

impl<Idx> Index<Idx> for StackSlice
where
  Idx: SliceIndex<[Value]>,
{
  type Output = Idx::Output;

  #[inline(always)]
  fn index(&self, index: Idx) -> &Self::Output {
    self.stack[self.range.clone()].index(index)
  }
}

impl<Idx> IndexMut<Idx> for StackSlice
where
  Idx: SliceIndex<[Value]>,
{
  #[inline]
  fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
    self.stack[self.range.clone()].index_mut(index)
  }
}
