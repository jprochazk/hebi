use core::fmt::{Debug, Display};

use super::list::List;
use crate::error::AllocError;
use crate::gc::{Gc, Object, Ref};
use crate::op::Reg;
use crate::val::Value;

pub struct Tuple {
  inner: Ref<List>,
}

impl Tuple {
  pub fn new(gc: &Gc, items: &[Value]) -> Result<Ref<Self>, AllocError> {
    let inner = List::try_with_capacity_in(gc, items.len())?;
    inner.extend_from_slice(gc, items)?;
    gc.try_alloc(Tuple { inner })
  }
}

impl Display for Tuple {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Debug::fmt(self, f)
  }
}

impl Debug for Tuple {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut f = f.debug_tuple("");
    for field in unsafe { self.inner.as_slice() } {
      f.field(field);
    }
    f.finish()
  }
}

impl Object for Tuple {
  const NEEDS_DROP: bool = false;
}

#[derive(Debug)]
pub struct TupleProto {
  start: Reg<u8>,
  count: u8,
}

impl TupleProto {
  pub fn new(gc: &Gc, start: Reg<u8>, count: u8) -> Result<Ref<Self>, AllocError> {
    gc.try_alloc(TupleProto { start, count })
  }

  #[inline]
  pub fn start(&self) -> Reg<u8> {
    self.start
  }

  #[inline]
  pub fn count(&self) -> u8 {
    self.count
  }
}

impl Display for TupleProto {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<tuple>")
  }
}

impl Object for TupleProto {}
