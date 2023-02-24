#![allow(clippy::wrong_self_convention)]

mod conv;
mod ctx;
mod emit;
mod isolate;
mod op;
mod util;
mod value;

/*

TODO: carefully design the public API
- Value
  - constructors
  - as_*
- Isolate
  - call
  - ?
*/

// TODO: everything that allocates should go through the context,
// eventually the context `alloc` method will use a faster allocator
// together with a garbage collector to make it worth the effort

use std::cell::{Ref, RefCell};
use std::fmt::{Debug, Display};

pub use conv::{FromHebi, IntoHebi, Value};
use ctx::Context;
use isolate::{Io, Isolate};
pub use value::object::Error as RuntimeError;
use value::Value as CoreValue;

pub type Result<T, E = RuntimeError> = std::result::Result<T, E>;

pub struct Hebi {
  isolate: RefCell<Isolate>,
}

// # Safety:
// Internally, the VM uses reference counting using the `Rc` type.
// Normally, it would be unsound to implement Send for something that
// uses `Rc`, but in this case, the user can *never* obtain an internal
// `Rc` (or equivalent). This means they can never clone that `Rc`, and
// then move their `Hebi` instance to another thread, causing a data race
// between the user's clone of the `Rc` and Hebi's clone.
//
// This is enforced by via the `conv::Value<'a>` type, which borrows from
// `Hebi`, meaning that `Hebi` may not be moved (potentially to another thread)
// while that value is borrowed.
unsafe impl Send for Hebi {}

impl Hebi {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self {
      isolate: RefCell::new(Isolate::new(Context::new())),
    }
  }

  pub fn with_io(io: impl Io) -> Self {
    Self {
      isolate: RefCell::new(Isolate::with_io(Context::new(), io)),
    }
  }

  pub fn check(&self, src: &str) -> Result<(), Vec<syntax::Error>> {
    syntax::parse(src)?;
    Ok(())
  }

  pub fn eval<'a, T: FromHebi<'a>>(&'a self, src: &str) -> Result<T, EvalError> {
    let module = syntax::parse(src)?;
    let module = emit::emit(self.isolate.borrow().ctx(), "code", &module).unwrap();
    let main = module.main();
    let result = self
      .isolate
      .borrow_mut()
      .call(main.into(), &[], CoreValue::none())?;
    let result = Value::bind(result);
    Ok(T::from_hebi(self, result)?)
  }

  pub fn io<T: 'static>(&self) -> Option<Ref<'_, T>> {
    match Ref::filter_map(self.isolate.borrow(), |isolate| {
      isolate.io().as_any().downcast_ref()
    }) {
      Ok(v) => Some(v),
      _ => None,
    }
  }
}

pub enum EvalError {
  Parse(Vec<syntax::Error>),
  Runtime(RuntimeError),
}

impl From<Vec<syntax::Error>> for EvalError {
  fn from(value: Vec<syntax::Error>) -> Self {
    EvalError::Parse(value)
  }
}
impl From<RuntimeError> for EvalError {
  fn from(value: RuntimeError) -> Self {
    EvalError::Runtime(value)
  }
}

impl Debug for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Parse(e) => f
        .debug_tuple("Parse")
        .field(&e.iter().map(|e| e.message.to_string()).collect::<Vec<_>>())
        .finish(),
      Self::Runtime(e) => f.debug_tuple("Runtime").field(&e.message).finish(),
    }
  }
}
impl Display for EvalError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use util::JoinIter;
    match self {
      EvalError::Parse(v) => write!(f, "syntax errors: {}", v.iter().join("; ")),
      EvalError::Runtime(v) => write!(f, "error: {}", v.message),
    }
  }
}
impl std::error::Error for EvalError {}

#[cfg(test)]
mod tests;
