#![allow(clippy::needless_lifetimes)]

pub mod map;
pub mod vec;

use bumpalo::Bump as Arena;

use crate::gc::{Alloc, Gc, NoAlloc};

pub type Vec<T, A> = allocator_api2::vec::Vec<T, A>;
pub type BumpVec<'arena, T> = Vec<T, &'arena Arena>;
pub type GcVec<'gc, T> = Vec<T, Alloc<'gc>>;
pub type GcVecN<T> = Vec<T, NoAlloc>;

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
