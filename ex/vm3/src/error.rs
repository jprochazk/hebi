#[cfg(not(feature = "std"))]
pub use core::error::Error as StdError;
#[rustfmt::skip]
#[cfg(feature = "std")]
pub use std::error::Error as StdError;

use crate::alloc;

pub type Result<T, E = alloc::boxed::Box<dyn StdError + Send + 'static>> =
  core::result::Result<T, E>;
