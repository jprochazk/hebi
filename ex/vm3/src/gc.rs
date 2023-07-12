//! # Garbage collector
//!
//! Supports two modes:
//! - `frame`
//! - `persist`
//!
//! ## Frame mode
//!
//! This mode is intended for use-cases like serving individual requests in
//! which you instantiate a new VM for each request in order to achieve total
//! request isolation.
//!
//! Memory is allocated with a fast bump allocator, and it is freed all at once
//! when the VM is dropped. This means it completely unsuitable for anything
//! long running- be careful, as you may find yourself easily running out of
//! memory, or having your program slow down to a halt due to the OS paging
//! memory in and out of disk.
//!
//! The bump allocator used is [bumpalo](https://github.com/fitzgen/bumpalo).
//!
//! ## Persist mode
//!
//! This mode is intended for longer-running tasks, where you'd otherwise have a
//! chance of running out of memory without any memory reclamation mechanism. An
//! example use-case is the main loop of a game, or a TCP listener accept loop.
//!
//! Memory is allocated and managed by a proper garbage collector.
//! The garbage collector is an implementation of
//! [this design](http://web.archive.org/web/20220107060536/http://wiki.luajit.org/New-Garbage-Collector)
//! by Mike Pall, intended for use in LuaJIT 3.
//!
//! NOTE: For the time being, the garbage collector uses a simpler tri-color
//! incremental algorithm. The plan is to eventually implement the full
//! quad-color algorithm.

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

pub mod bump;

pub trait Object: Debug + Display {}

pub trait Collector {
  type Typed<T: Object + 'static>;
  type Untyped;

  fn alloc<T: Object + 'static>(&self, obj: T) -> Self::Typed<T>;

  fn write<T: Object + 'static>(obj: &T);
  fn write_container<T: Object + 'static>(obj: &T);
}

// TODO:
pub struct Value<O> {
  _v: u64,
  _o: PhantomData<O>,
}

pub trait Tracer: private::Sealed {
  type Collector: Collector;

  fn object<T: Object + 'static>(&self, obj: <Self::Collector as Collector>::Typed<T>);
  fn value(&self, value: Value<<Self::Collector as Collector>::Untyped>);
}

/// # Safety
/// You must ensure that all fields which hold values or objects are passed to
/// the `tracer` instance.
///
/// This type should be automatically derived using the `#[object]` attribute
/// whenever possible. `#[object]` will also insert the correct write barriers.
pub unsafe trait Trace {
  unsafe fn trace<T>(&self, tracer: &T)
  where
    T: Tracer;
}

mod private {
  pub trait Sealed {}
}
