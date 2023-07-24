#[cfg(not(feature = "std"))]
pub use core::error::Error as StdError;
use core::fmt::Display;
#[rustfmt::skip]
#[cfg(feature = "std")]
pub use std::error::Error as StdError;

use crate::alloc;

pub type Result<T, E = alloc::boxed::Box<dyn StdError + Send + 'static>> =
  core::result::Result<T, E>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllocError;

impl Display for AllocError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "allocation failed")
  }
}

impl StdError for AllocError {}
macro_rules! impl_alloc_error_from {
  ($T:ty) => {
    impl From<$T> for AllocError {
      fn from(_: $T) -> Self {
        Self
      }
    }
  };
}

impl_alloc_error_from!(bumpalo::AllocErr);
impl_alloc_error_from!(bumpalo::collections::CollectionAllocErr);
impl_alloc_error_from!(hashbrown::TryReserveError);
impl_alloc_error_from!(allocator_api2::alloc::AllocError);
impl_alloc_error_from!(crate::alloc::collections::TryReserveError);
