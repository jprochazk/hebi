#[cfg(feature = "nanbox")]
mod nanbox;
#[cfg(feature = "nanbox")]
pub use nanbox::Value;

#[cfg(not(feature = "nanbox"))]
mod portable;
#[cfg(not(feature = "nanbox"))]
pub use portable::Value;

pub mod object;

use std::fmt::{Debug, Display};

pub use object::ptr::Ref;
use object::Object;

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
    } else if let Some(v) = v.to_object() {
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
    } else if let Some(v) = v.to_object() {
      // TODO: maybe also include inner object repr in the debug output
      f.debug_tuple("Object").field(&v.name()).finish()
    } else {
      unreachable!("invalid type");
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ctx::Context;
  use crate::util::JoinIter;
  use crate::value::object::Object;

  struct Bar {
    value: u64,
  }

  impl Object for Bar {
    fn name(&self) -> &'static str {
      "Bar"
    }
  }

  impl Debug for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.debug_struct("Bar").field("value", &self.value).finish()
    }
  }

  impl Display for Bar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      Debug::fmt(self, f)
    }
  }

  #[test]
  fn create_value() {
    let cx = Context::for_test();
    let values = [
      Value::float(std::f64::consts::PI),
      Value::int(-1_000_000),
      Value::bool(true),
      Value::none(),
      Value::object(cx.alloc(Bar { value: 100 })),
    ];
    let snapshot = format!("[{}]", values.iter().join(", "));
    assert_snapshot!(snapshot);
  }

  #[test]
  fn drop_object_value() {
    let cx = Context::for_test();
    Value::object(cx.alloc(Bar { value: 100 }));
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
    let cx = Context::for_test();

    // refcount = 1
    let ptr = cx.alloc(Bar { value: 100 }).into_any();
    assert_eq!(ptr.refs(), 1);

    // create a value from the pointer
    // refcount = 2
    let a = Value::object(ptr.clone());
    assert_eq!(ptr.refs(), 2);

    // clone it once
    // refcount = 3
    let b = a.clone();
    assert_eq!(ptr.refs(), 3);

    // check object refcounts
    let ptr_a = a.to_object().unwrap();
    assert_eq!(ptr_a.refs(), 3);

    let ptr_b = b.to_object().unwrap();
    assert_eq!(ptr_b.refs(), 3);

    // reconstruct and drop
    drop(Value::object(ptr_a));
    assert_eq!(ptr.refs(), 2);

    drop(Value::object(ptr_b));
    assert_eq!(ptr.refs(), 1);
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
