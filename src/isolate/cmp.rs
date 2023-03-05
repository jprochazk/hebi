pub use std::cmp::Ordering;

use super::*;
use crate::{Error, Result};

// TODO: metamethods - they should be checked for BEFORE getting here
// but the whole process would ideally be encapsulated here

pub fn partial_cmp(lhs: Value, rhs: Value) -> Result<Option<Ordering>> {
  if let (Some(lhs), Some(rhs)) = (lhs.clone().to_object_raw(), rhs.clone().to_object_raw()) {
    let lhs_addr = unsafe { lhs._get() as *const _ as usize };
    let rhs_addr = unsafe { rhs._get() as *const _ as usize };
    if lhs_addr == rhs_addr {
      return Ok(Some(Ordering::Equal));
    }

    unimplemented!("object comparison metamethods");
  }

  if let Some(lhs) = lhs.clone().to_int() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(lhs.partial_cmp(&rhs));
    } else if let Some(rhs) = rhs.clone().to_float() {
      return Ok((lhs as f64).partial_cmp(&rhs));
    }
  }

  if let Some(lhs) = lhs.clone().to_float() {
    if let Some(rhs) = rhs.clone().to_int() {
      return Ok(lhs.partial_cmp(&(rhs as f64)));
    } else if let Some(rhs) = rhs.clone().to_float() {
      return Ok(lhs.partial_cmp(&rhs));
    }
  }

  if let (Some(lhs), Some(rhs)) = (lhs.clone().to_bool(), rhs.clone().to_bool()) {
    return Ok(lhs.partial_cmp(&rhs));
  }

  if let (Some(lhs), Some(rhs)) = (lhs.to_none(), rhs.to_none()) {
    return Ok(lhs.partial_cmp(&rhs));
  }

  // TODO: span + print types
  Err(Error::runtime("cannot compare {lhs} and {rhs}"))
}
