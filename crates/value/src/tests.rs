use super::*;
use crate::object::Temp;

#[test]
fn create_value() {
  let values = [
    Value::float(std::f64::consts::PI),
    Value::int(-1_000_000),
    Value::bool(true),
    Value::none(),
    Value::object(Ptr::new()),
  ];
  insta::assert_debug_snapshot!(values);
}

#[test]
fn drop_object_value() {
  Value::object(Ptr::new());
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
  let rc = Rc::new(Temp(0u64));
  assert_eq!(Rc::strong_count(&rc), 1);

  // create a value from the pointer
  // refcount = 2
  let a = Value::object(Ptr(rc.clone()));
  assert_eq!(Rc::strong_count(&rc), 2);

  // clone it once
  // refcount = 3
  let b = a.clone();
  assert_eq!(Rc::strong_count(&rc), 3);

  // check object refcounts
  let rc_a = a.to_object().unwrap().0;
  assert_eq!(Rc::strong_count(&rc_a), 3);

  let rc_b = b.to_object().unwrap().0;
  assert_eq!(Rc::strong_count(&rc_b), 3);

  // reconstruct and drop
  let a = Value::object(Ptr(rc_a));
  let b = Value::object(Ptr(rc_b));

  drop(a);
  assert_eq!(Rc::strong_count(&rc), 2);

  drop(b);
  assert_eq!(Rc::strong_count(&rc), 1);
}
