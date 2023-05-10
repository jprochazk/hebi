#[macro_use]
mod macros;

use super::*;

check! {
  example,
  r#"
    v := 0
    v
  "#
}

check! {
  make_fn,
  r#"
    fn test():
      return "yo"

    test
  "#
}

check! {
  basic_fn_call,
  r#"
    fn test():
      return "yo"

    test()
  "#
}

check! {
  closure_call,
  r#"
    fn outer():
      v := "yo"
      fn inner():
        return v
      return inner
    
    outer()()
  "#
}

check! {
  make_fn_with_args,
  r#"
    fn test(v):
      return v
    
    test
  "#
}

check! {
  call_fn_with_args,
  r#"
    fn test(v):
      return v
    
    test(100)
  "#
}

check! {
  call_fn_with_args__error_not_enough_args,
  r#"
    fn test(v):
      return v
    
    test()
  "#
}

check! {
  call_fn_with_args__error_too_many_args,
  r#"
    fn test(v):
      return v
    
    test(100, 100)
  "#
}

check! {
  call_fn_recursive,
  r#"
    fn test(v):
      if v: return "yo"
      else: return test(true)
    
    test(true)
    test(false)
  "#
}
