pub use std::cmp::Ordering;

use super::*;
use crate::{Error, Result};

pub fn partial_cmp(lhs: Value, rhs: Value) -> Result<Option<Ordering>> {
  if let Some(lhs) = lhs.as_int() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(lhs.partial_cmp(&rhs));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok((lhs as f64).partial_cmp(&rhs));
    }
  } else if let Some(lhs) = lhs.as_float() {
    if let Some(rhs) = rhs.as_int() {
      return Ok(lhs.partial_cmp(&(rhs as f64)));
    } else if let Some(rhs) = rhs.as_float() {
      return Ok(lhs.partial_cmp(&rhs));
    }
  }

  // TODO: span + print types
  Err(Error::new("cannot compare values", 0..0))
}
