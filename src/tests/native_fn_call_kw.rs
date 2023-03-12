use derive::function;

#[function]
fn simple(#[kw] v: i32) -> String {
  format!("value: {v}")
}
check! {
  call_kw_simple,
  fns: [simple],
  r#"
    print simple(v=1)
  "#
}
check_error! {
  call_kw_simple_missing_args,
  fns: [simple],
  r#"
    print simple()
  "#
}
check_error! {
  call_kw_simple_too_many_args,
  fns: [simple],
  r#"
    print simple(v=1,test=2)
  "#
}

#[function]
fn with_default(
  #[kw] a: i32,
  #[kw]
  #[default(100)]
  b: i32,
) -> String {
  format!("{a}+{b}={}", a + b)
}
check! {
  call_kw_with_default,
  fns: [with_default],
  r#"
    print with_default(a=1)
    print with_default(a=1, b=2)
    print with_default(b=2, a=1)
  "#
}
check_error! {
  call_kw_with_default_missing_args,
  fns: [with_default],
  r#"
    print with_default()
  "#
}
check_error! {
  call_kw_with_default_too_many_args,
  fns: [with_default],
  r#"
    print with_default(a=1,b=2,c=3)
  "#
}
check_error! {
  call_kw_with_default_too_many_args2,
  fns: [with_default],
  r#"
    print with_default(a=1,b=2,c=3,d=4)
  "#
}

#[function]
fn with_default_2(
  #[kw]
  #[default(100)]
  b: i32,
  #[kw] a: i32,
) -> String {
  format!("{a}+{b}={}", a + b)
}
check! {
  call_kw_with_default_2,
  fns: [with_default_2],
  r#"
    print with_default_2(a=1)
    print with_default_2(a=1, b=2)
    print with_default_2(b=2, a=1)
  "#
}
check_error! {
  call_kw_with_default_2_missing_args,
  fns: [with_default_2],
  r#"
    print with_default_2()
  "#
}
check_error! {
  call_kw_with_default_2_too_many_args,
  fns: [with_default_2],
  r#"
    print with_default_2(a=1,b=2,c=3)
  "#
}
check_error! {
  call_kw_with_default_2_too_many_args2,
  fns: [with_default_2],
  r#"
    print with_default_2(a=1,b=2,c=3,d=4)
  "#
}
