use super::*;
use crate::value::ptr::Ptr;

#[test]
fn test_object() {
  let mut v = Object::list([0.into(), 1.into(), 2.into()]);
  v.as_list_mut().unwrap().push(3.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));

  let mut v = Object::dict([("a".into(), 0.into())]);
  v.as_dict_mut().unwrap().insert("b", 1);
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));
}

#[test]
fn test_object_ptr() {
  let mut v = Ptr::new(Object::list([0.into(), 1.into(), 2.into()]));
  v.get_mut().as_list_mut().unwrap().push(3.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));

  let mut v = Ptr::new(Object::dict([("a".into(), 0.into())]));
  v.get_mut().as_dict_mut().unwrap().insert("b", 1);
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));
}

#[test]
fn test_object_value() {
  let mut v = Value::from(Object::list([0.into(), 1.into(), 2.into()]));
  v.as_list_mut().unwrap().push(3.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));

  let mut v = Value::from(Object::dict([("a".into(), 0.into())]));
  v.as_dict_mut().unwrap().insert("b", 1);
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));
}
