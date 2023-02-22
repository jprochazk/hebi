check! {
  func,
  r#"
    fn f():
      print "test"

    f()
    f()
  "#
}
check! {
  func_with_pos,
  r#"
    fn f(a, b, c):
      print a, b, c

    f(0, 1, 2)
    f(0, 1, 2)
  "#
}
check_error! {
  func_not_enough_args,
  r#"
    fn f(a, b, c):
      print a, b, c

    f()
  "#
}
check! {
  func_with_argv,
  r#"
    fn f(a, *rest):
      print a, rest

    f(0)
    f(0, 1, 2)
  "#
}
