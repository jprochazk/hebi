use std::fmt::Display;
use std::ops::{Index, IndexMut};
use std::ptr::NonNull;
use std::slice::SliceIndex;

use super::func::func_name;
use super::handle::Handle;
use super::{Access, List};
use crate::value::constant::Constant;
use crate::value::Value;

#[derive(Clone)]
pub struct Frame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  #[allow(dead_code)]
  func: Value,
  pub code: NonNull<[u8]>,
  pub const_pool: NonNull<[Constant]>,
  pub frame_size: usize,
  pub captures: Option<NonNull<[Value]>>,

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
  pub fn new(func: Value, num_args: usize, on_return: OnReturn) -> Self {
    Self::with_stack(func, num_args, on_return, Stack::with_capacity(256))
  }

  pub fn with_stack(func: Value, num_args: usize, on_return: OnReturn, stack: Stack) -> Self {
    if let Some(f) = func.clone().to_method() {
      return Frame::with_stack(f.func(), num_args, on_return, stack);
    }

    let Parts {
      code,
      const_pool,
      frame_size,
      captures,
    } = get_parts(func.clone());

    let mut stack = stack;
    stack.extend(frame_size);

    Frame {
      func,
      code,
      const_pool,
      on_return,
      stack,
      captures,
      frame_size,
      num_args,
    }
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
  captures: Option<NonNull<[Value]>>,
}

fn get_parts(callable: Value) -> Parts {
  if let Some(mut func) = callable.clone().to_func() {
    let code = NonNull::from(unsafe { func.code_mut() });
    let const_pool = NonNull::from(func.const_pool());
    let frame_size = func.frame_size() as usize;
    let captures = None;
    return Parts {
      code,
      const_pool,
      frame_size,
      captures,
    };
  }

  if let Some(mut closure) = callable.clone().to_closure() {
    let mut func = closure.descriptor().func();
    let code = NonNull::from(unsafe { func.code_mut() });
    let const_pool = NonNull::from(func.const_pool());
    let frame_size = func.frame_size() as usize;
    let captures = Some(NonNull::from(unsafe { closure.captures_mut() }));
    return Parts {
      code,
      const_pool,
      frame_size,
      captures,
    };
  }

  panic!("cannot create frame from {callable}")
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
  pub fn new() -> Self {
    Self {
      inner: Handle::alloc(List::new()),
      base: 0,
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      inner: Handle::alloc(List::with_capacity(capacity)),
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

impl Default for Stack {
  fn default() -> Self {
    Self::new()
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
