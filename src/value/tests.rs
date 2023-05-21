use super::*;
use crate::object::{Object, Ptr};
use crate::util::JoinIter;
use crate::vm::global::Global;

struct Bar {
  value: u64,
}

impl Object for Bar {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "Bar"
  }
}

generate_vtable!(Bar);

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
  let global = Global::default();
  let values = [
    Value::float(std::f64::consts::PI),
    Value::int(-1_000_000),
    Value::bool(true),
    Value::none(),
    Value::object(global.alloc(Bar { value: 100 })),
  ];
  let snapshot = format!("[{}]", values.iter().join(", "));
  assert_snapshot!(snapshot);
}

#[test]
fn drop_object_value() {
  let global = Global::default();
  Value::object(global.alloc(Bar { value: 100 }));
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
  let global = Global::default();

  // refcount = 1
  let ptr = global.alloc(Bar { value: 100 }).into_any();
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
  let ptr_a = a.to_any().unwrap();
  assert_eq!(ptr_a.refs(), 3);

  let ptr_b = b.to_any().unwrap();
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
