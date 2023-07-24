use core::mem::transmute;

use bumpalo::Bump as Arena;

use super::{HasAlloc, HasNoAlloc};
use crate::gc::{Alloc, Gc, NoAlloc};

pub type Vec<T, A> = allocator_api2::vec::Vec<T, A>;
pub type BumpVec<'arena, T> = Vec<T, &'arena Arena>;
pub type GcVec<'gc, T> = Vec<T, Alloc<'gc>>;
pub type GcVecN<T> = Vec<T, NoAlloc>;

impl<T> HasNoAlloc for GcVecN<T> {
  type Alloc<'gc> = GcVec<'gc, T>;

  fn as_alloc<'gc>(&self, gc: &'gc Gc) -> &Self::Alloc<'gc> {
    let v: &Self::Alloc<'gc> = unsafe { transmute(self) };
    v.allocator().set(gc);
    v
  }

  fn as_alloc_mut<'gc>(&mut self, gc: &'gc Gc) -> &mut Self::Alloc<'gc> {
    let v: &mut Self::Alloc<'gc> = unsafe { transmute(self) };
    v.allocator().set(gc);
    v
  }

  fn to_alloc<'gc>(self, gc: &'gc Gc) -> Self::Alloc<'gc> {
    let v: Self::Alloc<'gc> = unsafe { transmute(self) };
    v.allocator().set(gc);
    v
  }
}

impl<'gc, T> HasAlloc for GcVec<'gc, T> {
  type NoAlloc = GcVecN<T>;

  fn as_no_alloc(&self) -> &Self::NoAlloc {
    unsafe { transmute(self) }
  }

  fn as_no_alloc_mut(&mut self) -> &mut Self::NoAlloc {
    unsafe { transmute(self) }
  }

  fn to_no_alloc(self) -> Self::NoAlloc {
    unsafe { transmute(self) }
  }
}
