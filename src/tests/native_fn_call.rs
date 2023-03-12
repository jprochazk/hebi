use derive::function;

#[function]
fn simple() -> String {
  "test".into()
}
check! {
  call_simple,
  fns: [simple],
  r#"
    print simple()
  "#
}

#[function]
fn with_pos(v: i32) -> String {
  format!("value: {v}")
}
check! {
  call_with_pos,
  fns: [with_pos],
  r#"
    print with_pos(1)
  "#
}
check_error! {
  call_with_pos_missing_args,
  fns: [with_pos],
  r#"
    print with_pos()
  "#
}
check_error! {
  call_with_pos_too_many_args,
  fns: [with_pos],
  r#"
    print with_pos(1, 2)
  "#
}

#[function]
fn with_pos_default(a: i32, #[default(100)] b: i32) -> String {
  format!("{a}+{b}={}", a + b)
}
check! {
  call_with_pos_default,
  fns: [with_pos_default],
  r#"
    print with_pos_default(1)
    print with_pos_default(1, 2)
  "#
}
check_error! {
  call_with_pos_default_missing_args,
  fns: [with_pos_default],
  r#"
    print with_pos_default()
  "#
}
check_error! {
  call_with_pos_default_too_many_args,
  fns: [with_pos_default],
  r#"
    print with_pos_default(1, 2, 3)
  "#
}
