check! {
  kw,
  r#"
    fn f(*, a):
      print a

    f(a=10)
  "#
}
check! {
  kw_order_independence,
  r#"
    fn f(*, a, b):
      print a, b

    f(a=1, b=2)
    f(b=2, a=1)
  "#
}
check! {
  kw_with_default,
  r#"
    fn f(*, a, b=10):
      print a, b

    f(a=1)
    f(a=1, b=2)
  "#
}
check! {
  kw_with_kwargs,
  r#"
    fn f(*, a, b, **kw):
      print a, b, kw

    f(a=1, b=2, c=3)
    f(c=3, b=2, a=1)
  "#
}
check! {
  kw_with_default_and_kawrgs,
  r#"
    fn f(*, a, b=10, **kw):
      print a, b, kw

    f(a=1, c=3)
    f(c=3, a=1)
    f(a=1, b=2, c=3)
    f(c=3, b=2, a=1)
  "#
}
check_error! {
  kw_missing,
  r#"
    fn f(*, a):
      print a

    f()
  "#
}
check_error! {
  kw_missing2,
  r#"
    fn f(*, a, b):
      print a, b

    f()
  "#
}
check_error! {
  kw_missing_with_kwargs,
  r#"
    fn f(*, a, b, **kw):
      print a, b, kw

    f()
  "#
}
check_error! {
  kw_unrecognized_and_missing,
  r#"
    fn f(*, a, b, **kw):
      print a, b, kw

    f(c=10)
  "#
}
