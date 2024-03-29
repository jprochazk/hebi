#[macro_use]
mod macros;

use super::*;
use crate::internal::syntax;

check!(print_int, r#"print 0"#);

check!(print_float, r#"print 2.5"#);

check!(print_string, r#"print "test""#);

check!(print_list, r#"print [0, 1, 2]"#);

check!(print_table, r#"print { a: 0, b: 1, c: 2 }"#);

check! {
  print_global,
  r#"
    v := 0
    print v
  "#
}

check! {
  print_module_var,
  as_module=true,
  r#"
    v := 0
    print v
  "#
}

check! {
  print_field,
  r#"
    v := { a: 0 }
    print v.a
    v.a = 1
  "#
}

check! {
  print_index,
  r#"
    v := { a: 0 }
    print v["a"]
    v["a"] = 1
  "#
}

check! {
  print_field_opt,
  r#"
    v := {}
    print ?v.a
    print ?v.a.b.c
  "#
}

check! {
  print_index_opt,
  r#"
    v := {}
    print ?v["a"]
    print ?v["a"]["b"]["c"]
  "#
}

check!(call_0, r#"f()"#);

check!(call_1, r#"f(0)"#);

check!(call_n, r#"f(0, 1, 2)"#);

check!(nested_call_0, r#"a(b(c()))"#);

check! {
  nested_call_subexpr,
  r#"
    0 + a(1 + b(2 + c(3 + 4, 5), 6), 7)
  "#
}

check!(call_arg_subexpr, r#"f(a+b)"#);

check! {
  function_no_params,
  r#"
    fn test():
      pass

    test()
  "#
}

check! {
  function_with_param,
  r#"
    fn test(a):
      print a

    test(0)
  "#
}

check! {
  function_with_default_param,
  r#"
    fn test(a, b=10):
      print a, b

    test(1)
    test(1, 2)
  "#
}

check! {
  generator_no_params,
  r#"
    fn test():
      yield "a"
      return "b"
    
    test()
  "#
}

check! {
  generator_with_param,
  r#"
    fn test(a):
      yield a
      return a
    
    test()
  "#
}

check! {
  generator_with_default_param,
  r#"
    fn test(a, b=10):
      yield a
      return b
    
    test()
  "#
}

check! {
  closure_call,
  r#"
    fn a():
      v := 0
      fn b():
        print v
      return b

    a()()
  "#
}

check! {
  closure,
  r#"
    fn a():
      v := 0
      fn b():
        fn c():
          fn d():
            print v
  "#
}

check! {
  conditional_exprs,
  r#"
    fn test0(a, b):
      a && b
    fn test1(a, b):
      a || b
    fn test2(a, b):
      a ?? b
  "#
}

check! {
  conditional_exprs_precedence,
  r#"
    fn test0(a, b, c, d):
      (a && b) || (c && d)
    fn test1(a, b, c, d):
      a && b || c && d
    fn test3(a, b, c, d):
      (a || (b && c)) || d
    fn test4(a, b, c, d):
      a || b && c || d
  "#
}

check! {
  opt_chaining,
  r#"
    fn f3(a, b, c, d):
      a ?? b ?? c
  "#
}

check! {
  if_stmt,
  r#"
    if true:
      print a
    elif true:
      print b
    else:
      print c
  "#
}

check! {
  if_stmt_var_resolution,
  r#"
    if a:
      b := a
      print b
    else:
      print b
  "#
}

check! {
  loop_print,
  r#"
    loop:
      print "test"
  "#
}

check! {
  loop_continue,
  r#"
    loop:
      continue
  "#
}
check! {
  loop_break,
  r#"
    loop:
      break
  "#
}

check! {
  while_print,
  r#"
    while true:
      print "test"
  "#
}
check! {
  while_continue,
  r#"
    while true:
      continue
  "#
}

check! {
  while_break,
  r#"
    while true:
      break
  "#
}

check! {
  while_print_0_to_10,
  r#"
    v := 0
    while v < 10:
      print "less than 10:", v
      v += 1
    print "now it's 10"
  "#
}

check! {
  while_nested_while_break,
  r#"
    while true:
      while true:
        break
      break
  "#
}

check! {
  loop_nested_loop_break,
  r#"
    loop:
      loop:
        break
      break
  "#
}

check! {
  while_nested_loop_break,
  r#"
    while true:
      loop:
        break
      break
  "#
}

check! {
  loop_nested_while_break,
  r#"
    loop:
      while true:
        break
      break
  "#
}

check! {
  while_nested_while,
  r#"
    while true:
      while true:
        continue
      continue
  "#
}

check! {
  loop_nested_loop,
  r#"
    loop:
      loop:
        continue
      continue
  "#
}

check! {
  while_nested_loop,
  r#"
    while true:
      loop:
        continue
      continue
  "#
}

check! {
  loop_nested_while,
  r#"
    loop:
      while true:
        continue
      continue
  "#
}

check! {
  for_range_0_to_10_print,
  r#"
    for i in 0..10:
      print i
  "#
}

check! {
  for_range_0_to_10_inclusive_print,
  r#"
    for i in 0..=10:
      print i
  "#
}

check! {
  for_range_0_to_10_inclusive_break,
  r#"
    for i in 0..=10:
      break
  "#
}

check! {
  for_range_0_to_10_inclusive_continue,
  r#"
    for i in 0..=10:
      continue
  "#
}

check! {
  for_iter_array,
  r#"
    a := [0, 1, 2]
    for v in a:
      print v
  "#
}

check!(method_call_0, r#"o.f()"#);

check!(method_call_1, r#"o.f(0)"#);

check!(method_call_n, r#"o.f(1,2,3)"#);

check! {
  empty_class,
  r#"
    class T: pass
  "#
}

check! {
  class_with_field,
  r#"
    class T:
      v = 0
  "#
}

check! {
  class_with_multiple_fields,
  r#"
    class T:
      a = 0
      b = 1
  "#
}

check! {
  class_with_field_and_method,
  r#"
    class T:
      v = 0
      fn test(self):
        print self.v
  "#
}

check! {
  class_with_field_and_closure_method,
  r#"
    u := 0
    class T:
      v = 0
      fn test(self):
        print self.v, u
  "#
}

check! {
  class_in_nested_scope_with_closure_method,
  r#"
    fn test():
      u := 0
      class T:
        v = 0
        fn test(self):
          print self.v, u
  "#
}

check! {
  empty_class_derived,
  r#"
    class T(U): pass
  "#
}

check! {
  class_derived_with_field,
  r#"
    class T(U):
      v = 0
  "#
}

check! {
  class_derived_with_multiple_fields,
  r#"
    class T(U):
      a = 0
      b = 1
  "#
}
check! {
  class_derived_with_field_and_method,
  r#"
    class T(U):
      v = 0
      fn test(self):
        print self.v
  "#
}
check! {
  class_derived_with_field_and_closure_method,
  r#"
    u := 0
    class T(U):
      v = 0
      fn test(self):
        print self.v, u
  "#
}
check! {
  class_derived_in_nested_scope_with_closure_method,
  r#"
    fn test():
      u := 0
      class T(U):
        v = 0
        fn test(self):
          print self.v, u
  "#
}

check! {
  class_instance,
  r#"
    class T: pass

    T()
  "#
}

check! {
  import_whole,
  r#"
    import test
    print test.symbol
  "#
}

check! {
  import_symbol,
  r#"
    from test import symbol
    print symbol
  "#
}

check! {
  import_symbol_multi,
  r#"
    from test import a, b
    print a, b
  "#
}

check! {
  import_multi,
  r#"
    from test.a0 import a1, a2
    from test.b0 import b1, b2
    print a1, a2
    print b1, b2
  "#
}

check! {
  fn_in_module,
  as_module=true,
  r#"
    value := 100
    fn set(v):
      value = v
    fn get():
      return value
  "#
}

check! {
  variable_scope_lifetime,
  r#"
    fn test():
      if true:
        v := 0
        if true:
          print v
          v := 0
          print v
        v := 0
        b := 0
        print b
  "#
}
