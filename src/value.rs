#[cfg(feature = "nanbox")]
mod nanbox;
#[cfg(feature = "nanbox")]
pub use nanbox::Value;

#[cfg(not(feature = "nanbox"))]
mod portable;
#[cfg(not(feature = "nanbox"))]
pub use portable::Value;

pub mod constant;

use std::fmt::{Debug, Display};

pub use crate::object::ptr::Ref;

impl Default for Value {
  fn default() -> Self {
    Self::none()
  }
}

impl Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let v = self.clone();
    if let Some(v) = v.clone().to_float() {
      write!(f, "{v}")?;
    } else if let Some(v) = v.clone().to_int() {
      write!(f, "{v}")?;
    } else if let Some(v) = v.clone().to_bool() {
      write!(f, "{v}")?;
    } else if v.is_none() {
      write!(f, "none")?;
    } else if let Some(v) = v.to_any() {
      write!(f, "{v}")?;
    } else {
      unreachable!("invalid type");
    }

    Ok(())
  }
}

impl Debug for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let v = self.clone();
    if let Some(v) = v.clone().to_float() {
      f.debug_tuple("Float").field(&v).finish()
    } else if let Some(v) = v.clone().to_int() {
      f.debug_tuple("Int").field(&v).finish()
    } else if let Some(v) = v.clone().to_bool() {
      f.debug_tuple("Bool").field(&v).finish()
    } else if v.is_none() {
      f.debug_tuple("None").finish()
    } else if let Some(v) = v.to_any() {
      f.debug_tuple("Object").field(&v).finish()
    } else {
      unreachable!("invalid type");
    }
  }
}

#[cfg(test)]
mod tests;
