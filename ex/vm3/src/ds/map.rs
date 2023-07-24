pub mod ord;

pub type HashMap<K, V, A> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>, A>;
pub type BumpHashMap<'arena, K, V> = HashMap<K, V, &'arena Arena>;
pub type GcHashMap<'gc, K, V> = HashMap<K, V, Alloc<'gc>>;
pub type GcHashMapN<K, V> = HashMap<K, V, NoAlloc>;
pub type OrdHashMap<K, V, A> = ord::OrdHashMap<K, V, A>;
pub type GcOrdHashMap<'gc, K, V> = ord::OrdHashMap<K, V, Alloc<'gc>>;
pub type GcOrdHashMapN<K, V> = ord::OrdHashMap<K, V, NoAlloc>;

pub fn fx() -> BuildHasherDefault<FxHasher> {
  BuildHasherDefault::default()
}

use core::hash::BuildHasherDefault;
use core::mem::transmute;

use bumpalo::Bump as Arena;
use rustc_hash::FxHasher;

use super::{HasAlloc, HasNoAlloc};
use crate::gc::{Alloc, Gc, NoAlloc};

impl<'gc, K, V> HasAlloc for GcHashMap<'gc, K, V> {
  type NoAlloc = GcHashMapN<K, V>;

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

impl<K, V> HasNoAlloc for GcHashMapN<K, V> {
  type Alloc<'gc> = GcHashMap<'gc, K, V>;

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

impl<'gc, K, V> HasAlloc for GcOrdHashMap<'gc, K, V> {
  type NoAlloc = GcOrdHashMapN<K, V>;

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

impl<K, V> HasNoAlloc for GcOrdHashMapN<K, V> {
  type Alloc<'gc> = GcOrdHashMap<'gc, K, V>;

  fn as_alloc<'gc>(&self, gc: &'gc Gc) -> &Self::Alloc<'gc> {
    let v: &Self::Alloc<'gc> = unsafe { transmute(self) };
    let (map, vec) = v.allocators();
    map.set(gc);
    vec.set(gc);
    v
  }

  fn as_alloc_mut<'gc>(&mut self, gc: &'gc Gc) -> &mut Self::Alloc<'gc> {
    let v: &mut Self::Alloc<'gc> = unsafe { transmute(self) };
    let (map, vec) = v.allocators();
    map.set(gc);
    vec.set(gc);
    v
  }

  fn to_alloc<'gc>(self, gc: &'gc Gc) -> Self::Alloc<'gc> {
    let v: Self::Alloc<'gc> = unsafe { transmute(self) };
    let (map, vec) = v.allocators();
    map.set(gc);
    vec.set(gc);
    v
  }
}
