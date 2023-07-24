pub type HashSet<T, A> = hashbrown::HashSet<T, BuildHasherDefault<FxHasher>, A>;
pub type BumpHashSet<'arena, T> = HashSet<T, &'arena Arena>;
pub type GcHashSet<'gc, T> = HashSet<T, Alloc<'gc>>;
pub type GcHashSetN<T> = HashSet<T, NoAlloc>;

use core::hash::BuildHasherDefault;
use core::mem::transmute;

use bumpalo::Bump as Arena;
use rustc_hash::FxHasher;

use super::{HasAlloc, HasNoAlloc};
use crate::gc::{Alloc, Gc, NoAlloc};

impl<'gc, T> HasAlloc for GcHashSet<'gc, T> {
  type NoAlloc = GcHashSetN<T>;

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

impl<T> HasNoAlloc for GcHashSetN<T> {
  type Alloc<'gc> = GcHashSet<'gc, T>;

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
