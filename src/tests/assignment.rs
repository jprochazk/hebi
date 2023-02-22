check! {
  assign,
  r#"
    v := 10
    v = 5
    print v
  "#
}
check! {
  add_assign,
  r#"
    v := 10
    v += 2
    print v
  "#
}
check! {
  sub_assign,
  r#"
    v := 10
    v -= 2
    print v
  "#
}
check! {
  mul_assign,
  r#"
    v := 10
    v *= 2
    print v
  "#
}
check! {
  div_assign,
  r#"
    v := 10
    v /= 2
    print v
  "#
}
check! {
  pow_assign,
  r#"
    v := 10
    v **= 2
    print v
  "#
}
check! {
  rem_assign,
  r#"
    v := 10
    v %= 2
    print v
  "#
}
check! {
  opt_assign_none,
  r#"
    v := none
    v ??= 2
    print v
  "#
}
check! {
  opt_assign_some,
  r#"
    v := 10
    v ??= 2
    print v
  "#
}
