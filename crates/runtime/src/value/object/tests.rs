use super::*;

#[test]
fn test_object() {
  let mut v = List::from([Value::int(0), Value::int(1), Value::int(2)]);
  v.push(Value::int(3));
  insta::assert_snapshot!(format!("display:\n{v}"));

  let mut v = Dict::from_iter([(Key::from("a"), Value::int(0))]);
  v.insert("b", Value::int(1));
  insta::assert_snapshot!(format!("display:\n{v}"));
}

#[test]
fn test_object_ptr() {
  let mut v = Handle::alloc(List::from([Value::int(0), Value::int(1), Value::int(2)]));
  v.push(Value::int(3));
  insta::assert_snapshot!(format!("display:\n{v}"));

  let mut v = Handle::alloc(Dict::from_iter([(Key::from("a"), Value::int(0))]));
  v.insert("b", Value::int(1));
  insta::assert_snapshot!(format!("display:\n{v}"));
}

#[test]
fn test_object_value() {
  let v = Value::object(Handle::alloc(List::from([
    Value::int(0),
    Value::int(1),
    Value::int(2),
  ])));
  v.clone().to_list().unwrap().push(Value::int(3));
  insta::assert_snapshot!(format!("display:\n{v}"));

  let v = Value::object(Handle::alloc(Dict::from_iter([(
    Key::from("a"),
    Value::int(0),
  )])));
  v.clone().to_dict().unwrap().insert("b", Value::int(1));
  insta::assert_snapshot!(format!("display:\n{v}"));
}
