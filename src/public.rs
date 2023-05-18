#![allow(clippy::new_without_default)]

#[macro_use]
mod macros;

use std::marker::PhantomData;

use self::value::{FromValuePack, ValueRef};
use crate::object::{Ptr, Table as OwnedTable};
use crate::vm::thread::{Args, Thread};
use crate::vm::Vm;

// public API
pub mod module;
pub mod object;
pub mod value;

pub use crate::error::{Error, Result};
pub use crate::fail;
pub use crate::object::module::Loader;
pub use crate::object::native::LocalBoxFuture;
pub use crate::public::module::NativeModule;
pub use crate::public::object::list::ListRef as List;
pub use crate::public::object::string::StringRef as Str;
pub use crate::public::object::table::TableRef as Table;
pub use crate::public::object::AnyRef as Any;
pub use crate::public::value::{FromValue, IntoValue, ValueRef as Value};

pub struct Hebi {
  vm: Vm,
}

// # Safety
// The VM uses reference counting similar to `Rc`, but without weak references.
// Reference counts are *not* atomic, which means that the VM is not thread
// safe. To make it safe to implement `Send`, we completely lock down the public
// API of the VM to ensure very limited access to values. Values are never given
// out as *owned*, they are always *borrowed*. This means that thread safety is
// ensured via the borrow checker as opposed to a `!Send` bound.
//
// In summary:
// - User cannot obtain owned `Rc<T>` from the VM
// - User cannot clone the VM and move it to another thread
//
// Thus it should be safe even if the reference counts are not atomic, as they
// will never be accessed from two or more threads at the same time.
unsafe impl Send for Hebi {}

impl Hebi {
  pub fn new() -> Self {
    Self { vm: Vm::new() }
  }

  pub fn eval<'cx>(&'cx mut self, code: &str) -> Result<ValueRef<'cx>> {
    self.vm.eval(code).map(|v| unsafe { v.bind_raw::<'cx>() })
  }

  pub async fn eval_async<'cx>(&'cx mut self, code: &str) -> Result<ValueRef<'cx>> {
    self
      .vm
      .eval_async(code)
      .await
      .map(|v| unsafe { v.bind_raw::<'cx>() })
  }

  pub fn globals(&self) -> Globals {
    Globals {
      table: self.vm.root.global.globals().clone(),
      lifetime: core::marker::PhantomData,
    }
  }

  pub fn scope(&mut self) -> Scope {
    Scope::new(&self.vm.root, Args::empty())
  }

  pub fn register(&mut self, module: &NativeModule) {
    self.vm.register(module)
  }
}

#[derive(Clone)]
pub struct Context<'cx> {
  #[allow(dead_code)] // will be used eventually in `IntoValue`/`FromValue` impls
  pub(crate) inner: crate::ctx::Context,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

pub struct Scope<'cx> {
  pub(crate) thread: Thread,
  pub(crate) args: Args,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

impl<'cx> Scope<'cx> {
  pub(crate) fn new(parent: &Thread, args: Args) -> Self {
    let thread = Thread::new(parent.cx.clone(), parent.global.clone(), parent.stack);
    Scope {
      thread,
      args,
      lifetime: PhantomData,
    }
  }

  pub fn cx(&self) -> Context<'cx> {
    Context {
      inner: self.thread.cx.clone(),
      lifetime: PhantomData,
    }
  }

  pub fn num_args(&self) -> usize {
    self.args.count
  }

  pub fn params<T: FromValuePack<'cx>>(&self) -> Result<T::Output> {
    let stack = unsafe { self.thread.stack.as_ref() };
    let args = &stack.regs[self.args.start..self.args.start + self.args.count];
    T::from_value_pack(args, self.cx())
  }

  pub fn param<T: FromValue<'cx>>(&self, n: usize) -> Result<T> {
    let stack = unsafe { self.thread.stack.as_ref() };
    let value = stack
      .regs
      .get(self.args.start + n)
      .cloned()
      .ok_or_else(|| error!("missing argument {n}"))?;
    let value = unsafe { value.bind_raw::<'cx>() };
    T::from_value(value, self.cx())
  }

  pub fn call(&mut self, value: ValueRef<'cx>, args: &[ValueRef<'cx>]) -> Result<ValueRef<'cx>> {
    self
      .thread
      .call(value.unbind(), <_>::unbind_slice(args))
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub async fn call_async(
    &mut self,
    value: ValueRef<'cx>,
    args: &[ValueRef<'cx>],
  ) -> Result<ValueRef<'cx>> {
    self
      .thread
      .call_async(value.unbind(), <_>::unbind_slice(args))
      .await
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn globals(&self) -> Globals {
    Globals {
      table: self.thread.global.globals().clone(),
      lifetime: core::marker::PhantomData,
    }
  }
}

pub struct Globals<'cx> {
  pub(crate) table: Ptr<OwnedTable>,
  lifetime: core::marker::PhantomData<&'cx ()>,
}

impl<'cx> Globals<'cx> {
  pub fn get(&self, key: &str) -> Option<ValueRef<'cx>> {
    self
      .table
      .get(key)
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn set(&self, key: &str, value: ValueRef<'cx>) {
    self.table.set(key, value.unbind());
  }
}

/// # Safety
/// - `T` must be `#[repr(C)]`
/// - `T` must have only one non-ZST field (`<T as Unbind>::Owned`)
pub(crate) unsafe trait IsSimpleRef: Sized {}

pub(crate) trait Bind: Sized {
  type Ref<'cx>: IsSimpleRef;

  unsafe fn bind_raw<'cx>(self) -> Self::Ref<'cx>;

  fn bind<'cx>(self, scope: Context<'cx>) -> Self::Ref<'cx> {
    let _ = scope;
    unsafe { Self::bind_raw::<'cx>(self) }
  }

  fn bind_raw_slice<'a, 'cx>(slice: &'a [Self]) -> &'a [Self::Ref<'cx>] {
    unsafe { std::mem::transmute::<&[Self], &[Self::Ref<'cx>]>(slice) }
  }

  fn bind_slice<'a, 'cx>(slice: &'a [Self], scope: Context<'cx>) -> &'a [Self::Ref<'cx>] {
    let _ = scope;
    Self::bind_raw_slice(slice)
  }
}

pub(crate) trait Unbind: Sized + IsSimpleRef {
  type Owned;

  fn unbind(self) -> Self::Owned;
  fn unbind_slice(slice: &[Self]) -> &[Self::Owned] {
    // Safe due to `IsSimpleRef`
    unsafe { std::mem::transmute::<&[Self], &[Self::Owned]>(slice) }
  }
}
