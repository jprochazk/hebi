#![allow(clippy::unusual_byte_groupings)]

pub mod object;
pub mod ptr;
pub mod util;

use std::hash::{Hash, Hasher};

use object::handle::Handle;
use object::ObjectHandle;
use ptr::*;

// TODO: remove Hash and Eq from `Value`, only `Key` needs to implement those
// directly, rest can be implemented at the VM level

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

impl From<f64> for Value {
  fn from(value: f64) -> Self {
    Value::float(value)
  }
}

impl From<i32> for Value {
  fn from(value: i32) -> Self {
    Value::int(value)
  }
}

impl From<bool> for Value {
  fn from(value: bool) -> Self {
    Value::bool(value)
  }
}

impl From<()> for Value {
  fn from(_: ()) -> Self {
    Value::none()
  }
}

impl<T> From<T> for Value
where
  object::Object: From<T>,
{
  fn from(value: T) -> Self {
    Value::object(Ptr::new(object::Object::from(value)))
  }
}

impl<T> From<Option<T>> for Value
where
  Value: From<T>,
{
  fn from(value: Option<T>) -> Self {
    match value {
      Some(value) => Self::from(value),
      None => Self::none(),
    }
  }
}

impl From<Ptr<object::Object>> for Value {
  fn from(value: Ptr<object::Object>) -> Self {
    Value::object(value)
  }
}

impl<T> From<Handle<T>> for Value
where
  T: ObjectHandle,
{
  fn from(value: Handle<T>) -> Self {
    Value::object(value.widen())
  }
}

impl std::fmt::Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let v = self.clone();
    if let Some(v) = v.as_float() {
      write!(f, "{v}")?;
    } else if let Some(v) = v.as_int() {
      write!(f, "{v}")?;
    } else if let Some(v) = v.as_bool() {
      write!(f, "{v}")?;
    } else if v.is_none() {
      write!(f, "none")?;
    } else if let Some(v) = v.as_object() {
      write!(f, "{v}")?;
    } else {
      unreachable!("invalid type");
    }

    Ok(())
  }
}

impl std::fmt::Debug for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let v = self.clone();
    let mut s = f.debug_struct("Value");
    if let Some(v) = v.as_float() {
      s.field("type", &"float");
      s.field("value", &v);
    } else if let Some(v) = v.as_int() {
      s.field("type", &"int");
      s.field("value", &v);
    } else if let Some(v) = v.as_bool() {
      s.field("type", &"bool");
      s.field("value", &v);
    } else if v.is_none() {
      s.field("type", &"none");
      s.field("value", &"<none>");
    } else if let Some(v) = v.as_object() {
      s.field("type", &"object");
      s.field("value", &v);
    } else {
      unreachable!("invalid type");
    }
    s.finish()
  }
}

#[cfg(test)]
mod tests {
  use object::Object;

  use super::*;

  fn object() -> Object {
    Object::string("test")
  }

  #[test]
  fn create_value() {
    let values = [
      Value::float(std::f64::consts::PI),
      Value::int(-1_000_000),
      Value::bool(true),
      Value::none(),
      Value::object(Ptr::new(object())),
    ];
    insta::assert_debug_snapshot!(values);
  }

  #[test]
  fn drop_object_value() {
    Value::object(Ptr::new(object()));
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
    let ptr = Ptr::new(object());
    assert_eq!(Ptr::strong_count(&ptr), 1);

    // create a value from the pointer
    // refcount = 2
    let a = Value::object(ptr.clone());
    assert_eq!(Ptr::strong_count(&ptr), 2);

    // clone it once
    // refcount = 3
    let b = a.clone();
    assert_eq!(Ptr::strong_count(&ptr), 3);

    // check object refcounts
    let ptr_a = a.into_object().unwrap();
    assert_eq!(Ptr::strong_count(&ptr_a), 3);

    let ptr_b = b.into_object().unwrap();
    assert_eq!(Ptr::strong_count(&ptr_b), 3);

    // reconstruct and drop
    let a = Value::object(ptr_a);
    let b = Value::object(ptr_b);

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
