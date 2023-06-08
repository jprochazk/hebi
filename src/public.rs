#![allow(clippy::new_without_default)]

#[macro_use]
mod macros;

use std::cell::RefMut;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;

use futures_util::TryFutureExt;

use self::value::FromValuePack;
use crate::error::{Error, Result};
use crate::object::function::Disassembly;
use crate::object::native::NativeClassInstance;
use crate::object::{table, Ptr, Type};
use crate::value::Value as OwnedValue;
use crate::vm;
use crate::vm::global::{Input, Output};
use crate::vm::thread::{Args, Slot0, Thread};
use crate::vm::{global, Config, Vm};

// public API
pub mod module;
pub mod object;
pub mod value;

pub use beef::lean::Cow;

pub use crate::fail;
pub use crate::object::module::ModuleLoader;
pub use crate::object::native::LocalBoxFuture;
pub use crate::public::module::NativeModule;
pub use crate::public::object::list::List;
pub use crate::public::object::string::Str;
pub use crate::public::object::table::Table;
pub use crate::public::object::Any;
pub use crate::public::value::{FromValue, IntoValue, Value};

#[cfg(feature = "serde")]
pub mod serde;
#[cfg(feature = "serde")]
pub use crate::public::serde::ValueDeserializer;

#[derive(Default)]
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

struct ForceSendFuture<F: Future<Output = Result<OwnedValue>>> {
  fut: F,
}
impl<F: Future<Output = Result<OwnedValue>>> ForceSendFuture<F> {
  pub unsafe fn new(fut: F) -> Self {
    Self { fut }
  }
}
unsafe impl<F: Future<Output = Result<OwnedValue>>> Send for ForceSendFuture<F> {}
impl<F> Future for ForceSendFuture<F>
where
  F: Future<Output = Result<OwnedValue>>,
{
  type Output = F::Output;

  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    let this = unsafe { self.get_unchecked_mut() };
    let fut = unsafe { Pin::new_unchecked(&mut this.fut) };
    fut.poll(cx)
  }
}

pub struct HebiBuilder<M, I, O> {
  module_loader: Option<Box<dyn crate::object::module::ModuleLoader>>,
  input: Option<Box<dyn crate::vm::global::Input>>,
  output: Option<Box<dyn crate::vm::global::Output>>,
  __: PhantomData<(M, I, O)>,
}

pub struct HasModuleLoader {
  __: (),
}
impl<I, O> HebiBuilder<(), I, O> {
  pub fn module_loader(
    self,
    module_loader: impl ModuleLoader + 'static,
  ) -> HebiBuilder<HasModuleLoader, I, O> {
    HebiBuilder {
      module_loader: Some(Box::new(module_loader)),
      input: self.input,
      output: self.output,
      __: PhantomData,
    }
  }
}

pub struct HasInput {
  __: (),
}
impl<M, O> HebiBuilder<M, (), O> {
  pub fn input(self, input: impl Input + 'static) -> HebiBuilder<M, HasInput, O> {
    HebiBuilder {
      module_loader: self.module_loader,
      input: Some(Box::new(input)),
      output: self.output,
      __: PhantomData,
    }
  }
}

pub struct HasOutput {
  __: (),
}
impl<M, I> HebiBuilder<M, I, ()> {
  pub fn output(self, output: impl Output + 'static) -> HebiBuilder<M, I, HasOutput> {
    HebiBuilder {
      module_loader: self.module_loader,
      input: self.input,
      output: Some(Box::new(output)),
      __: PhantomData,
    }
  }
}

impl<M, I, O> HebiBuilder<M, I, O> {
  pub fn finish(self) -> Hebi {
    Hebi {
      vm: Vm::with_config(Config {
        module_loader: self.module_loader,
        input: self.input,
        output: self.output,
      }),
    }
  }
}

impl Hebi {
  pub fn new() -> Self {
    Self { vm: Vm::default() }
  }

  pub fn builder() -> HebiBuilder<(), (), ()> {
    HebiBuilder {
      module_loader: None,
      input: None,
      output: None,
      __: PhantomData,
    }
  }

  pub fn eval<'cx, 'src>(&'cx mut self, code: &'src str) -> Result<Value<'cx>>
  where
    'src: 'cx,
  {
    pollster::block_on(self.eval_async(code))
  }

  pub fn eval_async<'cx, 'src>(
    &'cx mut self,
    code: &'src str,
  ) -> impl Future<Output = Result<Value<'cx>>> + Send + 'cx
  where
    'src: 'cx,
  {
    let fut = self.vm.eval(code);
    unsafe { ForceSendFuture::new(fut) }.map_ok(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn compile<'cx>(&self, code: &str) -> Result<Chunk<'cx>> {
    self.vm.compile(code).map(|chunk| Chunk {
      inner: chunk,
      lifetime: PhantomData,
    })
  }

  pub fn run<'cx>(&'cx mut self, chunk: Chunk<'cx>) -> Result<Value<'cx>> {
    pollster::block_on(self.run_async(chunk))
  }

  pub fn run_async<'cx>(
    &'cx mut self,
    chunk: Chunk<'cx>,
  ) -> impl Future<Output = Result<Value<'cx>>> + Send + 'cx {
    let fut = self.vm.entry(chunk.inner);
    unsafe { ForceSendFuture::new(fut) }.map_ok(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn global(&self) -> Global {
    Global {
      inner: self.vm.root.global.clone(),
      lifetime: PhantomData,
    }
  }

  pub fn register(&mut self, module: &NativeModule) {
    self.vm.register(module)
  }
}

impl Debug for Hebi {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Hebi").finish()
  }
}

#[derive(Clone)]
pub struct Chunk<'cx> {
  pub(crate) inner: vm::Chunk,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

impl<'cx> Chunk<'cx> {
  pub fn disassemble(&self) -> Disassembly {
    self.inner.disassemble()
  }
}

#[derive(Clone)]
pub struct Global<'cx> {
  pub(crate) inner: global::Global,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

impl<'cx> Global<'cx> {
  pub fn get(&self, key: &str) -> Option<Value<'cx>> {
    self
      .inner
      .get(key)
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub fn set(&self, key: Str<'cx>, value: Value<'cx>) {
    self.inner.set(key.unbind(), value.unbind());
  }

  pub fn print(&self, f: impl Display) -> Result<()> {
    write!(&mut self.inner.io().output.borrow_mut(), "{f}").map_err(Error::user)
  }

  pub fn println(&self, f: impl Display) -> Result<()> {
    writeln!(&mut self.inner.io().output.borrow_mut(), "{f}").map_err(Error::user)
  }

  pub fn output(&mut self) -> RefMut<'_, dyn Output> {
    RefMut::map(self.inner.io().output.borrow_mut(), |output| {
      output.as_mut()
    })
  }

  pub fn input(&mut self) -> RefMut<'_, dyn Input> {
    RefMut::map(self.inner.io().input.borrow_mut(), |input| input.as_mut())
  }

  pub fn entries<'a>(&'a self) -> GlobalEntries<'a, 'cx> {
    GlobalEntries {
      entries: self.inner.entries(),
      lifetime: PhantomData,
    }
  }
}

pub struct GlobalEntries<'a, 'cx> {
  entries: table::Entries<'a>,
  lifetime: PhantomData<&'cx ()>,
}

impl<'a, 'cx> Iterator for GlobalEntries<'a, 'cx> {
  type Item = (Str<'cx>, Value<'cx>);

  fn next(&mut self) -> Option<Self::Item> {
    self
      .entries
      .next()
      .map(|(key, value)| unsafe { (key.bind_raw::<'cx>(), value.bind_raw::<'cx>()) })
  }
}

#[derive(Clone)]
pub struct Scope<'cx> {
  pub(crate) thread: Thread,
  pub(crate) stack_base: usize,
  pub(crate) args: Args,
  pub(crate) lifetime: PhantomData<&'cx ()>,
}

impl<'cx> Scope<'cx> {
  pub(crate) fn new(parent: &Thread, stack_base: usize, args: Args) -> Self {
    debug_assert!(unsafe { parent.stack.as_ref() }.regs.len() >= args.start + args.count);
    let thread = Thread::new(parent.global.clone(), parent.stack);
    Scope {
      thread,
      stack_base,
      args,
      lifetime: PhantomData,
    }
  }

  pub(crate) fn alloc<T: Type>(&self, v: T) -> Ptr<T> {
    self.thread.global.alloc(v)
  }

  pub(crate) fn intern(&self, s: impl Into<Cow<'static, str>>) -> Ptr<crate::object::Str> {
    self.thread.global.intern(s)
  }

  pub fn global(&self) -> Global<'cx> {
    Global {
      inner: self.thread.global.clone(),
      lifetime: PhantomData,
    }
  }

  pub fn num_args(&self) -> usize {
    self.args.count
  }

  pub fn params<T: FromValuePack<'cx>>(&self) -> Result<T::Output> {
    let stack = unsafe { self.thread.stack.as_ref() };
    let range = self.args.start..self.args.start + self.args.count;
    let Some(args) = stack.regs.get(range) else {
      fail!("expected {} args, got {}", T::len(), self.args.count);
    };
    T::from_value_pack(args, self.global())
  }

  pub fn param<T: FromValue<'cx>>(&self, n: usize) -> Result<T> {
    let stack = unsafe { self.thread.stack.as_ref() };
    let index = self.args.start + n;
    let Some(value) = stack.regs.get(index).cloned() else {
      fail!("missing argument {n}");
    };
    let value = unsafe { value.bind_raw::<'cx>() };
    T::from_value(value, self.global())
  }

  // TODO: does this also need to be force-Send?
  pub async fn call<'a>(
    &'a mut self,
    value: Any<'cx>,
    args: &'a [Value<'cx>],
  ) -> Result<Value<'cx>> {
    self
      .thread
      .call(value.unbind(), <_>::unbind_slice(args))
      .await
      .map(|value| unsafe { value.bind_raw::<'cx>() })
  }

  pub(crate) fn consume_args(&mut self, n: usize) {
    self.args.start += n;
    self.args.count -= n;
  }

  pub(crate) fn enter_nested(
    &mut self,
    slot0: Slot0,
    args: Args,
    frame_size: Option<usize>,
  ) -> Scope<'cx> {
    self
      .thread
      .enter_nested_scope(self.stack_base, slot0, args, frame_size)
  }

  pub(crate) fn leave(mut self) {
    self.thread.truncate_stack(self.stack_base);
  }
}

impl<'cx> Global<'cx> {
  pub fn new_instance<T: Send + 'static>(&self, value: T) -> Result<Value<'cx>> {
    let instance = match self.inner.get_type::<T>() {
      Some(ty) => NativeClassInstance {
        instance: Box::new(value),
        class: ty,
      },
      None => fail!("`{}` is not a registered type", std::any::type_name::<T>()),
    };
    let instance = OwnedValue::object(self.inner.alloc(instance));
    Ok(unsafe { instance.bind_raw::<'cx>() })
  }
}

impl<'cx> Scope<'cx> {
  pub fn new_instance<T: Send + 'static>(&self, value: T) -> Result<Value<'cx>> {
    self.global().new_instance(value)
  }
}

impl Hebi {
  pub fn new_instance<T: Send + 'static>(&self, value: T) -> Result<Value> {
    self.global().new_instance(value)
  }
}

pub struct This<'cx, T: Send> {
  pub(crate) inner: Ptr<NativeClassInstance>,
  lifetime: PhantomData<&'cx T>,
}

impl<'cx, T: Send + 'static> This<'cx, T> {
  pub fn new(inner: Ptr<NativeClassInstance>) -> Option<Self> {
    if !inner.instance.is::<T>() {
      return None;
    }
    Some(This {
      inner,
      lifetime: PhantomData,
    })
  }
}

impl<'cx, T: Send + 'static> Deref for This<'cx, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    debug_assert!(self.inner.instance.is::<T>());
    unsafe { self.inner.instance.downcast_ref().unwrap_unchecked() }
  }
}

/// # Safety
/// - `T` must be `#[repr(C)]`
/// - `T` must have only one non-ZST field (`<T as Unbind>::Owned`)
pub(crate) unsafe trait IsSimpleRef: Sized {}

pub(crate) trait Bind: Sized {
  type Ref<'cx>: IsSimpleRef;

  unsafe fn bind_raw<'cx>(self) -> Self::Ref<'cx>;

  fn bind<'cx>(self, global: Global<'cx>) -> Self::Ref<'cx> {
    let _ = global;
    unsafe { Self::bind_raw::<'cx>(self) }
  }

  fn bind_raw_slice<'a, 'cx>(slice: &'a [Self]) -> &'a [Self::Ref<'cx>] {
    unsafe { std::mem::transmute::<&[Self], &[Self::Ref<'cx>]>(slice) }
  }

  fn bind_slice<'a, 'cx>(slice: &'a [Self], global: Global<'cx>) -> &'a [Self::Ref<'cx>] {
    let _ = global;
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
