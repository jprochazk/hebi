use super::*;

macro_rules! check {
  ($input:literal) => {{
    let input = indoc::indoc!($input);
    let module = match syntax::parse(input) {
      Ok(module) => module,
      Err(e) => {
        for err in e {
          eprintln!("{}", err.report(input));
        }
        panic!("Failed to parse source, see errors above.")
      }
    };
    let ctx = Context::new();
    let chunk = match emit(ctx, "[[main]]", &module) {
      Ok(chunk) => chunk,
      Err(e) => {
        panic!("failed to emit chunk:\n{}", e.report(input));
      }
    };
    let snapshot = format!(
      "# Input:\n{input}\n\n# Chunk:\n{}",
      chunk.disassemble(op::instruction::disassemble, false)
    );
    insta::assert_snapshot!(snapshot);
  }};
}

#[test]
fn print_literals() {
  check!(r#"print 0"#);
  check!(r#"print 2.5"#);
  check!(r#"print "test""#);
  check!(r#"print [0, 1, 2]"#);
  check!(r#"print { a: 0, b: 1, c: 2 }"#);
}

#[test]
fn print_variable() {
  check! {
    r#"
      v := 0
      print v # load_reg
    "#
  }
  // TODO: capture + nested capture
  check! {
    r#"
      print v # load_global
    "#
  }
}

#[test]
fn print_field() {
  check! {
    r#"
      v := { a: 0 }
      print v.a
      v.a = 1
    "#
  }
  check! {
    r#"
      v := { a: 0 }
      print v["a"]
      v["a"] = 1
    "#
  }
  check! {
    r#"
      v := {}
      print ?v.a
      print ?v.a.b.c
    "#
  }
  check! {
    r#"
      v := {}
      print ?v["a"]
      print ?v["a"]["b"]["c"]
    "#
  }
}

#[test]
fn call() {
  check!(r#"f()"#);
  check!(r#"f(0)"#);
  check!(r#"f(0, 1, 2)"#);
  check!(r#"f(a=0)"#);
  check!(r#"f(a=0, b=1, c=2)"#);
  check!(r#"f(a, b, c=2)"#);
  check!(r#"a(b(c()))"#);
}

#[test]
fn func() {
  check! {
    r#"
      fn test():
        pass

      test()
    "#
  }

  check! {
    r#"
      fn test(a):
        print a
      
      test(0)
    "#
  }

  check! {
    r#"
      fn test(a, b=10):
        print a, b

      test(1)
      test(1, 2)
    "#
  }

  check! {
    r#"
      fn test(a, *, b):
        print a, b

      test(1, b=2)
    "#
  }

  check! {
    r#"
      fn test(a, *, b=10):
        print a, b

      test(1)
      test(1, b=2)
    "#
  }

  check! {
    r#"
      fn test(a, *v, b=10, **kw):
        print a, v, b, kw

      test(1, 2)
      test(1, 2, b=3, c=4)
    "#
  }
}

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

#[test]
fn logical_expr() {
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
}

#[test]
fn if_stmt() {
  check! {
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
    r#"
      if a:
        b := a
        print b
      else:
        print b
    "#
  }
}
