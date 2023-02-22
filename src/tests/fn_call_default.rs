check! {
  default_pos,
  r#"
    fn f(a=10):
      print a
    
    f()
    f(1)
  "#
}
check! {
  pos_and_default_pos,
  r#"
    fn f(a, b=10):
      print a, b
    
    f(1)
    f(1,2)
  "#
}
check_error! {
  default_pos_too_many_args,
  r#"
    fn f(a=10):
      print a
    
    f(1,2)
  "#
}
check_error! {
  pos_and_default_pos_too_many_args,
  r#"
    fn f(a, b=10):
      print a
    
    f(1,2,3)
  "#
}
check_error! {
  pos_and_default_pos_not_enough_args,
  r#"
    fn f(a, b=10):
      print a
    
    f()
  "#
}
check! {
  pos_and_default_and_argv,
  r#"
    fn f(a, b=10, *c):
      print a, b, c
    
    f(1)
    f(1,2)
    f(1,2,3)
  "#
}
