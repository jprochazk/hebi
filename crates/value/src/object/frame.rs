use std::ops::RangeBounds;
use std::ptr::NonNull;

use super::handle::Handle;
use super::List;
use crate::ptr::Ref;
use crate::Value;

#[derive(Clone)]
pub struct Frame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  #[allow(dead_code)]
  func: Value,
  pub code: NonNull<[u8]>,
  pub const_pool: NonNull<[Value]>,
  pub frame_size: usize,
  pub captures: Option<NonNull<[Value]>>,

  pub pc: usize,
  // set by `call`
  pub return_address: usize,
  pub stack: Stack,
  pub num_args: usize,
}

impl Frame {
  /// Create a new call frame.
  ///
  /// # Panics
  ///
  /// If `func` is not a function, closure, or method.
  pub fn new(func: Value, num_args: usize) -> Self {
    Self::with_stack(func, num_args, Stack::with_capacity(256))
  }

  pub fn with_stack(mut func: Value, num_args: usize, mut stack: Stack) -> Self {
    if let Some(f) = func.as_method() {
      return Frame::new(f.func.clone(), num_args);
    }

    let Parts {
      code,
      const_pool,
      frame_size,
      captures,
    } = get_parts(&mut func);

    stack.extend(frame_size);

    Frame {
      func,
      code,
      const_pool,
      pc: 0,
      return_address: 0,
      stack,
      captures,
      frame_size,
      num_args,
    }
  }

  pub fn stack_base(&self) -> usize {
    self.stack.base
  }
}

struct Parts {
  code: NonNull<[u8]>,
  const_pool: NonNull<[Value]>,
  frame_size: usize,
  captures: Option<NonNull<[Value]>>,
}

fn get_parts(f: &mut Value) -> Parts {
  if let Some(mut f) = f.as_func_mut() {
    let code = NonNull::from(f.code_mut());
    let const_pool = NonNull::from(f.const_pool());
    let frame_size = f.frame_size() as usize;
    let captures = None;
    return Parts {
      code,
      const_pool,
      frame_size,
      captures,
    };
  }

  if let Some(mut f) = f.as_closure_mut() {
    let code = NonNull::from(f.code_mut().as_mut());
    let const_pool = NonNull::from(f.const_pool().as_ref());
    let frame_size = f.frame_size() as usize;
    let captures = Some(NonNull::from(&mut f.captures[..]));
    return Parts {
      code,
      const_pool,
      frame_size,
      captures,
    };
  }

  panic!("cannot create frame from {f}")
}

impl Drop for Frame {
  fn drop(&mut self) {
    self.stack.truncate(self.stack_base())
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
      inner: List::new().into(),
      base: 0,
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      inner: List::with_capacity(capacity).into(),
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
    let mut inner = self.inner.borrow_mut();
    inner.extend((0..n).map(|_| Value::none()));
  }

  pub fn truncate(&mut self, len: usize) {
    let mut inner = self.inner.borrow_mut();
    inner.truncate(len)
  }

  pub fn get(&self, index: usize) -> Ref<'_, Value> {
    Ref::map(self.inner.borrow(), |v| &v[self.base + index])
  }

  pub fn set(&mut self, index: usize, value: Value) {
    self.inner.borrow_mut()[self.base + index] = value;
  }

  pub fn base(&self) -> usize {
    self.base
  }

  pub fn slice<R>(&self, range: R) -> Ref<'_, [Value]>
  where
    R: RangeBounds<usize>,
  {
    let start = self.base
      + match range.start_bound() {
        std::ops::Bound::Included(v) => *v,
        std::ops::Bound::Excluded(v) => (*v) + 1,
        std::ops::Bound::Unbounded => 0,
      };
    let end = self.base
      + match range.end_bound() {
        std::ops::Bound::Included(v) => (*v) + 1,
        std::ops::Bound::Excluded(v) => *v,
        std::ops::Bound::Unbounded => self.inner.borrow().len(),
      };
    Ref::map(self.inner.borrow(), |v| &v[start..end])
  }
}

impl Default for Stack {
  fn default() -> Self {
    Self::new()
  }
}

impl std::fmt::Debug for Frame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Frame").field("func", &self.func).finish()
  }
}
