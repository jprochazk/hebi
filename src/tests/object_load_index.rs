check! {
  field,
  r#"
    v := { a: 10 }
    print v["a"]
  "#
}
check_error! {
  unknown,
  r#"
    v := {}
    print v["a"]
  "#
}
check! {
  unknown_opt,
  r#"
    v := {}
    print ?v["a"]
  "#
}
check_error! {
  unknown_opt_then_error,
  r#"
    v := {}
    print ?v["a"]
    print v["a"]
  "#
}
check! {
  unknown_opt_nested,
  r#"
    v := { a: { b: 10 } }
    print ?v["a"]["b"]
  "#
}
check_error! {
  nested_unknown,
  r#"
    v := { a: {} }
    print v["a"]["b"]
  "#
}
check! {
  nested_unknown_opt,
  r#"
    v := { a: {} }
    print ?v["a"]["b"]
  "#
}
