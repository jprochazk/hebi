pub use std::cmp::Ordering;

use super::*;
use crate::{Result, RuntimeError};

pub fn partial_cmp(lhs: Value, rhs: Value) -> Result<Option<Ordering>> {
  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(lhs.partial_cmp(&rhs));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok((lhs as f64).partial_cmp(&rhs));
    }
  } else if let Some(lhs) = lhs.to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(lhs.partial_cmp(&(rhs as f64)));
    } else if let Some(rhs) = rhs.to_float() {
      return Ok(lhs.partial_cmp(&rhs));
    }
  }

  // TODO: span + print types
  Err(RuntimeError::script("cannot compare values", 0..0))
}
