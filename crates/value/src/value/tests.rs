use super::*;
use crate::object::Object;

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
  let ptr_a = a.to_object().unwrap();
  assert_eq!(Ptr::strong_count(&ptr_a), 3);

  let ptr_b = b.to_object().unwrap();
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
  Value::float(f64::from_bits(super::mask::QNAN)); // quiet nans will panic
}