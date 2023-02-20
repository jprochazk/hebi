#![allow(clippy::unusual_byte_groupings)]

pub mod constant;
pub mod object;
pub mod ptr;

use object::handle::Handle;
use object::ObjectType;
use ptr::*;

#[cfg(not(feature = "portable"))]
#[path = "impl/nanbox.rs"]
mod value_impl;

#[cfg(feature = "portable")]
#[path = "impl/portable.rs"]
mod value_impl;

pub use value_impl::Value;

impl Default for Value {
  fn default() -> Self {
    Self::none()
  }
}

impl std::fmt::Display for Value {
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
    } else if let Some(v) = v.to_object_raw() {
      write!(f, "{}", unsafe { v._get() })?;
    } else {
      unreachable!("invalid type");
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::util::JoinIter;
  use crate::value::object::{Object, Str};

  #[test]
  fn create_value() {
    let values = [
      Value::float(std::f64::consts::PI),
      Value::int(-1_000_000),
      Value::bool(true),
      Value::none(),
      Value::object(Handle::alloc(Str::from("test"))),
    ];
    let snapshot = format!("[{}]", values.iter().join(", "));
    insta::assert_snapshot!(snapshot);
  }

  #[test]
  fn drop_object_value() {
    Value::object(Handle::alloc(Str::from("test")));
  }

  #[test]
  fn clone_and_drop_values() {
    let values = [
      Value::float(std::f64::consts::PI),
      Value::int(-1_000_000),
      Value::bool(true),
      Value::none(),
    ];

    for value in values.iter() {
      std::hint::black_box(value.clone());
    }
  }

  #[test]
  fn clone_and_drop_object_value() {
    // refcount = 1
    let ptr = Ptr::alloc(Object::from(Str::from("test")));
    assert_eq!(Ptr::strong_count(&ptr), 1);

    // create a value from the pointer
    // refcount = 2
    let a = Value::object_raw(ptr.clone());
    assert_eq!(Ptr::strong_count(&ptr), 2);

    // clone it once
    // refcount = 3
    let b = a.clone();
    assert_eq!(Ptr::strong_count(&ptr), 3);

    // check object refcounts
    let ptr_a = a.to_object_raw().unwrap();
    assert_eq!(Ptr::strong_count(&ptr_a), 3);

    let ptr_b = b.to_object_raw().unwrap();
    assert_eq!(Ptr::strong_count(&ptr_b), 3);

    // reconstruct and drop
    let a = Value::object_raw(ptr_a);
    let b = Value::object_raw(ptr_b);

    drop(a);
    assert_eq!(Ptr::strong_count(&ptr), 2);

    drop(b);
    assert_eq!(Ptr::strong_count(&ptr), 1);
  }

  #[test]
  #[should_panic]
  fn create_value_from_qnan() {
    // TODO: how else do you create a quiet nan?
    // quiet nans will panic
    Value::float(f64::from_bits(
      0b01111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000,
    ));
  }
}
