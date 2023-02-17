#[path = "common/common.rs"]
#[macro_use]
mod common;

check! {
  field,
  r#"
    class T:
      a = 0
    v := T(a=10)
    print v.a
  "#
}
check_error! {
  unknown,
  r#"
    class T: pass
    v := T()
    print v.a
  "#
}
check! {
  unknown_opt,
  r#"
    class T: pass
    v := T()
    print ?v.a
  "#
}
check_error! {
  unknown_opt_then_error,
  r#"
    class T: pass
    v := T()
    print ?v.a
    print v.a
  "#
}
check! {
  nested_opt,
  r#"
    class T:
      b = 0
    class U:
      a = T()
    v := U(a=T(b=10))
    print ?v.a.b
  "#
}
check_error! {
  nested_unknown_error,
  r#"
    class T:
      a = none
    v := T()
    print v.a.b
  "#
}
check! {
  nested_unknown_opt,
  r#"
    class T:
      a = none
    v := T()
    print ?v.a.b
  "#
}
