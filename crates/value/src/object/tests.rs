use super::*;
use crate::Ptr;

#[test]
fn test_object() {
  let mut v = Object::list([0.into(), 1.into(), 2.into()]);
  v.as_list_mut().unwrap().push(3.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));

  let mut v = Object::dict([("a".into(), 0.into())]);
  v.as_dict_mut().unwrap().insert("b".into(), 1.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));
}

#[test]
fn test_object_ptr() {
  let v = Ptr::new(Object::list([0.into(), 1.into(), 2.into()]));
  v.borrow_mut().as_list_mut().unwrap().push(3.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));

  let v = Ptr::new(Object::dict([("a".into(), 0.into())]));
  v.borrow_mut()
    .as_dict_mut()
    .unwrap()
    .insert("b".into(), 1.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));
}

#[test]
fn test_object_value() {
  let mut v = Value::from(Object::list([0.into(), 1.into(), 2.into()]));
  v.as_list_mut().unwrap().push(3.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));

  let mut v = Value::from(Object::dict([("a".into(), 0.into())]));
  v.as_dict_mut().unwrap().insert("b".into(), 1.into());
  insta::assert_snapshot!(format!("display:\n{v}\n\ndebug:\n{v:#?}"));
}
