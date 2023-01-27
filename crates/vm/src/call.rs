use std::ptr::NonNull;

use value::{Object, Ptr, Value};

pub(crate) struct CallFrame {
  // ensures that the pointers below remain valid for the lifetime of the `CallFrame`
  pub(crate) func: Ptr<Object>,
  pub(crate) code: NonNull<[u8]>,
  pub(crate) const_pool: NonNull<[Value]>,
  pub(crate) captures: Option<NonNull<[Value]>>,
  pub(crate) base: usize,
}

impl CallFrame {
  /// Create a new call frame.
  ///
  /// # Panics
  ///
  /// If `func` is not a function or closure.
  fn new(func: Ptr<Object>, base: usize) -> Self {
    if let Some(f) = func.borrow_mut().as_func_mut() {
      let code = NonNull::from(f.code_mut());
      let const_pool = NonNull::from(f.const_pool());
      Self {
        func: func.clone(),
        code,
        const_pool,
        captures: None,
        base,
      }
    } else if let Some(f) = func.borrow_mut().as_closure_mut() {
      let code = NonNull::from(f.code_mut().as_mut());
      let const_pool = NonNull::from(f.const_pool().as_ref());
      let captures = NonNull::from(&mut f.captures[..]);
      Self {
        func: func.clone(),
        code,
        const_pool,
        captures: Some(captures),
        base,
      }
    } else {
      panic!("attempted to create call frame from something that is not a function: {func:?}");
    }
  }
}
