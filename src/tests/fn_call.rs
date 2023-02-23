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
check! {
  func_recursive,
  r#"
    fn fac(n):
      if n < 2:
        return n
      else:
        return n * fac(n-1)
    
    print fac(5)
    print fac(5)
  "#
}
