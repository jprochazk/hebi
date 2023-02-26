use super::*;
use crate::ctx::Context;

macro_rules! check {
  ($input:literal) => {{
    let ctx = Context::new();
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
    let result = match Emitter::new(ctx.clone(), "code", &module, true).emit_main() {
      Ok(result) => result,
      Err(e) => {
        panic!("failed to emit func:\n{}", e.report(input));
      }
    };
    let tracking = result.regalloc.get_tracking();
    let tracking = tracking.borrow();
    let func = ctx.alloc(result.func);
    let snapshot = format!(
      "# Input:\n{input}\n\n# Func:\n{}\n\n# Regalloc:\n{}",
      func.disassemble(false),
      crate::emit::regalloc::DisplayTracking(&tracking),
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
  check!(r#"f(a+b, c=a+b)"#);
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

#[test]
fn loop_stmt() {
  check! {
    r#"
      loop:
        print "test"
    "#
  }
  check! {
    r#"
      loop:
        continue
    "#
  }
  check! {
    r#"
      loop:
        break
    "#
  }

  check! {
    r#"
      while true:
        print "test"
    "#
  }
  check! {
    r#"
      while true:
        continue
    "#
  }
  check! {
    r#"
      while true:
        break
    "#
  }
  check! {
    r#"
      v := 0
      while v < 10:
        print "less than 10:", v
        v += 1
      print "now it's 10"
    "#
  }

  check! {
    r#"
      while true:
        while true:
          break
        break
    "#
  }
  check! {
    r#"
      loop:
        loop:
          break
        break
    "#
  }
  check! {
    r#"
      while true:
        loop:
          break
        break
    "#
  }
  check! {
    r#"
      loop:
        while true:
          break
        break
    "#
  }

  check! {
    r#"
      while true:
        while true:
          continue
        continue
    "#
  }
  check! {
    r#"
      loop:
        loop:
          continue
        continue
    "#
  }
  check! {
    r#"
      while true:
        loop:
          continue
        continue
    "#
  }
  check! {
    r#"
      loop:
        while true:
          continue
        continue
    "#
  }
}

#[test]
fn for_loop_stmt() {
  check! {
    r#"
      for i in 0..10:
        print i
    "#
  }
  check! {
    r#"
      for i in 0..=10:
        print i
    "#
  }
  check! {
    r#"
      for i in 0..=10:
        break
    "#
  }
  check! {
    r#"
      for i in 0..=10:
        continue
    "#
  }
}

#[test]
fn method_call() {
  check!(r#"o.f()"#);
  check!(r#"o.f(0)"#);
  check!(r#"o.f(1,2,3)"#);
  check!(r#"o.f(1,2,c=3)"#);
}

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

#[test]
fn imports() {
  check! {
    r#"
      fn main():
        import test

        print test.symbol
    "#
  }
  check! {
    r#"
      fn main():
        from test import symbol

        print symbol
    "#
  }
  check! {
    r#"
      fn main():
        from test import a, b

        print a, b
    "#
  }
  check! {
    r#"
      fn main():
        from test.a0 import a1, a2
        from test.b0 import b1, b2

        print a1, a2
        print b1, b2
    "#
  }
  check! {
    r#"
      import test

      print test.value
    "#
  }
}

/* #[test]
fn _temp() {
  check! {
    r#"
      fn test2():
        print "test (without self)"
      class Test2:
        test = test2
    "#
  }
} */
