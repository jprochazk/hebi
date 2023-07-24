#![allow(clippy::needless_lifetimes)]

pub mod map;
pub mod set;
pub mod vec;

use core::hash::BuildHasherDefault;

use rustc_hash::FxHasher;

use crate::gc::Gc;

pub fn fx() -> BuildHasherDefault<FxHasher> {
  BuildHasherDefault::default()
}

pub trait HasNoAlloc {
  type Alloc<'gc>;

  fn as_alloc<'gc>(&self, gc: &'gc Gc) -> &Self::Alloc<'gc>;
  fn as_alloc_mut<'gc>(&mut self, gc: &'gc Gc) -> &mut Self::Alloc<'gc>;
  fn to_alloc<'gc>(self, gc: &'gc Gc) -> Self::Alloc<'gc>;
}

pub trait HasAlloc {
  type NoAlloc;

  fn as_no_alloc(&self) -> &Self::NoAlloc;
  fn as_no_alloc_mut(&mut self) -> &mut Self::NoAlloc;
  fn to_no_alloc(self) -> Self::NoAlloc;
}
