use super::*;
use crate::ctx::Context;
use crate::syntax;

macro_rules! check {
  ($name:ident, $(as_module=$as_module:expr,)? $input:literal) => {
      #[allow(unused_mut, unused_assignments)]
      #[test]
      fn $name() {
      let mut as_module = false;
      $(as_module = $as_module;)?
      let cx = Context::for_test();
      let input = indoc::indoc!($input);
      let module = match syntax::parse(&cx, input) {
        Ok(module) => module,
        Err(e) => {
          for err in e {
            eprintln!("{}", err.report(input, true));
          }
          panic!("Failed to parse source, see errors above.")
        }
      };
      let module = emit(&cx, &module, "main", !as_module);
      let snapshot = format!(
        "# Input:\n{input}\n\n# Func:\n{}\n\n",
        module.root.disassemble(),
      );
      assert_snapshot!(snapshot);
    }
  };
}

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

/*
#[test]
fn closure() {
  check! {
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
    r#"
      fn a():
        v := 0
        fn b():
          fn c():
            fn d():
              print v
    "#
  }
}
*/
/*
check! {
  r#"
    fn f0(a, b):
      a && b
      a || b
      a ?? b


    fn f1_a(a, b, c, d):
      (a && b) || (c && d)
    fn f1_b(a, b, c, d):
        a && b  ||  c && d

    fn f2_a(a, b, c, d):
      (a || (b && c)) || d
    fn f2_b(a, b, c, d):
      a ||  b && c  || d

    fn f3(a, b, c, d):
      a ?? b ?? c
  "#
}
*/

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
  for_iter_0_to_10_print,
  r#"
    for i in 0..10:
      print i
  "#
}

check! {
  for_iter_0_to_10_inclusive_print,
  r#"
    for i in 0..=10:
      print i
  "#
}

check! {
  for_iter_0_to_10_inclusive_break,
  r#"
    for i in 0..=10:
      break
  "#
}

check! {
  for_iter_0_to_10_inclusive_continue,
  r#"
    for i in 0..=10:
      continue
  "#
}

check!(method_call_0, r#"o.f()"#);

check!(method_call_1, r#"o.f(0)"#);

check!(method_call_n, r#"o.f(1,2,3)"#);

/*
#[test]
fn class_def() {
  check! {
    r#"
      class T: pass
    "#
  }
  check! {
    r#"
      class T:
        v
    "#
  }
  check! {
    r#"
      class T:
        v = 0
    "#
  }
  check! {
    r#"
      class T:
        a = 0
        b = 1
    "#
  }
  check! {
    r#"
      class T:
        v = 0
        fn test(self):
          print self.v
    "#
  }
  check! {
    r#"
      u := 0
      class T:
        v = 0
        fn test(self):
          print self.v, u
    "#
  }
  check! {
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
    r#"
      class T(U): pass
    "#
  }
  check! {
    r#"
      class T(U):
        v
    "#
  }
  check! {
    r#"
      class T(U):
        v = 0
    "#
  }
  check! {
    r#"
      class T(U):
        a = 0
        b = 1
    "#
  }
  check! {
    r#"
      class T(U):
        v = 0
        fn test(self):
          print self.v
    "#
  }
  check! {
    r#"
      u := 0
      class T(U):
        v = 0
        fn test(self):
          print self.v, u
    "#
  }
  check! {
    r#"
      fn test():
        u := 0
        class T(U):
          v = 0
          fn test(self):
            print self.v, u
    "#
  }
}

#[test]
fn class_instance() {
  check! {
    r#"
      class T: pass

      T()
    "#
  }
}
*/

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

/*
#[test]
fn fn_in_module() {
  check! {
    as_module=true,
    r#"
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  }
}

#[test]
fn meta_methods() {
  check! {
    r#"
      class U: pass
      class T(U):
        value = 0
        fn @add(self, other): pass
        fn add(self, other): pass
    "#
  }
} */
