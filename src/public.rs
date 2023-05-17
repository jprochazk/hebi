#![allow(clippy::new_without_default)]

#[macro_use]
mod macros;

use std::marker::PhantomData;

use self::module::NativeModule;
use self::value::ValueRef;
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
pub use crate::public::object::list::ListRef as List;
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
    Scope {
      thread: &mut self.vm.root,
      args: Args { start: 0, count: 0 },
    }
  }

  pub fn register(&mut self, module: &NativeModule) {
    self.vm.register(module)
  }
}

#[derive(Clone)]
pub struct Context<'cx> {
  #[allow(dead_code)] // will be used eventually in `IntoValue`/`FromValue` impls
  pub(crate) cx: crate::ctx::Context,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

pub struct Scope<'cx> {
  pub(crate) thread: &'cx mut Thread,
  pub(crate) args: Args,
}

impl<'cx> Scope<'cx> {
  pub fn cx(&self) -> Context<'cx> {
    Context {
      cx: self.thread.cx.clone(),
      lifetime: PhantomData,
    }
  }

  pub fn num_args(&self) -> usize {
    self.args.count
  }

  pub fn argument(&self, n: usize) -> Option<ValueRef> {
    self
      .thread
      .stack
      .get(self.args.start + n)
      .cloned()
      .map(|v| unsafe { v.bind_raw::<'cx>() })
  }

  pub fn call(&mut self, value: ValueRef<'cx>, args: &[ValueRef<'cx>]) -> Result<ValueRef<'cx>> {
    self
      .thread
      .call(value.unbind(), <_>::unbind_slice(args))
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn globals(&self) -> Globals {
    Globals {
      table: self.thread.global.globals().clone(),
      lifetime: core::marker::PhantomData,
    }
  }
}

pub struct AsyncScope<'cx> {
  pub(crate) thread: Thread,
  pub(crate) args: Args,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

impl<'cx> AsyncScope<'cx> {
  pub fn cx(&self) -> Context<'cx> {
    Context {
      cx: self.thread.cx.clone(),
      lifetime: PhantomData,
    }
  }

  pub fn num_args(&self) -> usize {
    self.args.count
  }

  pub fn argument(&self, n: usize) -> Option<ValueRef> {
    self
      .thread
      .stack
      .get(self.args.start + n)
      .cloned()
      .map(|v| unsafe { v.bind_raw::<'cx>() })
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
