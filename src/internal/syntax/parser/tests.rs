use indoc::indoc;

use super::*;
use crate::internal::syntax::lexer::Lexer;
use crate::internal::vm::global::Global;

// TODO: emit input expression in snapshot
// do this for all snapshots tests that don't do it already

macro_rules! check_module {
  ($input:literal) => {
    let global = Global::default();
    let input = indoc!($input);
    match parse(global, input) {
      Ok(module) => assert_debug_snapshot!(module),
      Err(e) => {
        for err in e.errors() {
          eprintln!("{}", err.report(input, true));
        }
        panic!("Failed to parse source, see errors above.")
      }
    };
  };
  ($name:ident, $input:literal) => {
    #[test]
    fn $name() {
      check_module!($input);
    }
  };
}

macro_rules! check_expr {
  ($input:literal) => {
    let global = Global::default();
    let input = $input;
    match Parser::new(global, Lexer::new(input)).expr() {
      Ok(module) => assert_debug_snapshot!(module),
      Err(err) => {
        eprintln!("{}", err.report(input, true));
        panic!("Failed to parse source, see errors above.")
      }
    };
  };
}

macro_rules! check_error {
  ($input:literal) => {
    let global = Global::default();
    let input = indoc!($input);
    match parse(global, input) {
      Ok(_) => panic!("module parsed successfully"),
      Err(e) => {
        let mut errors = String::new();
        for err in e.errors() {
          errors += &err.report(input, true);
          errors += "\n";
        }
        assert_snapshot!(errors);
      }
    };
  };
  ($name:ident, $input:literal) => {
    #[test]
    fn $name() {
      check_error!($input);
    }
  };
}

#[test]
fn import_stmt() {
  check_module! {
    r#"#!hebi
      import module
      from module import z
      from module import x, y, z

      import module.nested
      from module.nested import z
      from module.nested import x, y, z
    "#
  };

  check_module! {
    r#"#!hebi
      import module as temp
      from module import z as temp
      from module import x as temp, y as temp, z as temp
      
      import module.nested as temp
      from module.nested import z as temp
      from module.nested import x as temp, y as temp, z as temp
    "#
  };

  check_error! {
    r#"#!hebi
      import a
        import b
    "#
  };

  check_error! {
    r#"#!hebi
      from m import a
        from m import b
    "#
  };

  check_error! {
    r#"#!hebi
      from
        m
    "#
  };

  check_error! {
    r#"#!hebi
      from m import
        a
    "#
  };

  check_error! {
    r#"#!hebi
      from m import a,
        b
    "#
  };

  check_error! {
    r#"#!hebi
      from m
        .b
    "#
  };
}

#[test]
fn binary_expr() {
  check_expr!(r#"a + b"#);
  check_expr!(r#"a - b"#);
  check_expr!(r#"a / b"#);
  check_expr!(r#"a ** b"#);
  check_expr!(r#"a * b"#);
  check_expr!(r#"a % b"#);
  check_expr!(r#"a == b"#);
  check_expr!(r#"a != b"#);
  check_expr!(r#"a > b"#);
  check_expr!(r#"a >= b"#);
  check_expr!(r#"a < b"#);
  check_expr!(r#"a <= b"#);
  check_expr!(r#"a is b"#);
  check_expr!(r#"a in b"#);
  check_expr!(r#"a && b"#);
  check_expr!(r#"a || b"#);
  check_expr!(r#"a ?? b"#);

  check_module! {
    r#"#!hebi
      a + b
      c + d
    "#
  };

  check_error! {
    r#"#!hebi
      a +
        b
    "#
  }
}

#[test]
fn unary_expr() {
  check_expr!(r#"+a"#);
  check_expr!(r#"-a"#);
  check_expr!(r#"!a"#);
  check_expr!(r#"?a.b[c].d()"#);
}

#[test]
fn postfix_expr() {
  check_expr!(r#"a.b[c].d"#);
  check_module! {
    r#"#!hebi
      a.b[c].d
      a.b[c].d
    "#
  };

  check_error! {
    r#"#!hebi
      a
      .b[c].d
    "#
  }
  check_error! {
    r#"#!hebi
      a.b[c]
      .d
    "#
  }
  check_error! {
    r#"a."#
  }
}

#[test]
fn call_expr() {
  check_expr!(r#"a(b, c,)"#);
  check_module! {
    r#"#!hebi
      a(b, c,)
      a(
        b,
      c, d
          ,
          )
    "#
  };
}

#[test]
fn simple_literal_expr() {
  check_module! {
    r#"#!hebi
      none
      true
      false
      1
      0.1
      1.5e3
      3.14e-3
      "\tas\\df\u{2800}\x28\n"
    "#
  }
}

#[test]
fn array_literal_expr() {
  check_module! {
    r#"#!hebi
      [0, 1, 2]
      [0,
       1,
       2,]
      [
        0,
        1,
        2,
      ]
    "#
  }
}

#[test]
fn table_literal_expr() {
  check_module! {
    r#"#!hebi
      {a:b, c:d, e:f}
      {a:b,
        c:d,
        e:f,}
      {
        a:b,
        c:d,
        e:f,
      }
    "#
  }

  check_module! {
    r#"#!hebi
      {[a]:b, [c]:d, [e]:f}
      {[a]:b,
       [c]:d,
       [e]:f,}
      {
        [a]:b,
        [c]:d,
        [e]:f,
      }
    "#
  }
}

#[test]
fn grouping_expr() {
  check_module! {
    r#"#!hebi
      # asdf
      (a + b)
      (
      a
       +
          b
      )
      (a
        (b))
      ((((a))))
    "#
  }
}

#[test]
fn assign_expr() {
  check_module! {
    r#"#!hebi
      # asdf
      a = b
      a := b
      a += b
      a -= b
      a /= b
      a *= b
      a %= b
      a **= b
      a ??= b
    "#
  }

  check_module! {
    r#"#!hebi
      # asdf
      a.b = b
      a.b += b
      a.b -= b
      a.b /= b
      a.b *= b
      a.b %= b
      a.b **= b
      a.b ??= b
    "#
  }

  check_error! {
    r#"#!hebi
      a
        = b
    "#
  }
  check_error! {
    r#"#!hebi
      a =
        b
    "#
  }

  check_error! {
    r#"a.b := c"#
  }

  check_error! {
    r#"a() = b"#
  }
}

#[test]
fn if_stmt() {
  check_module! {
    r#"#!hebi
      if a: pass
      elif b: pass
      elif c: pass
      else: pass
    "#
  }

  check_module! {
    r#"#!hebi
      if a:
        pass
      elif b:
        pass
      elif c:
        pass
      else:
        pass
    "#
  }

  check_module! {
    r#"#!hebi
      if a:
        a
        b
      elif b:
        a
        b
      elif c:
        a
        b
      else:
        a
        b
    "#
  }

  check_module! {
    r#"#!hebi
      if a:
        if b:
          pass
    "#
  }

  check_error! {
    r#"#!hebi
      if a:
        a
          b
      else: pass
    "#
  }

  check_error! {
    r#"#!hebi
      if a
        : pass
      else: pass
    "#
  }

  check_error! {
    r#"#!hebi
      if a: pass
      elif b
        : pass
      else: pass
    "#
  }

  check_error! {
    r#"#!hebi
      if a: pass
      elif b: pass
      else
        : pass
    "#
  }

  check_error! {
    r#"#!hebi
      if a: pass
      elif b: pass
        else: pass
    "#
  }

  check_error! {
    r#"#!hebi
      if a: pass
        elif b: pass
      else: pass
    "#
  }

  check_module! {
    r#"#!hebi
      if (
        some_very_long_condition &&
        with_many_sub_expressions() ||
        (which_are_complex.too())
      ):
        pass
    "#
  }
}

#[test]
fn loop_stmts() {
  check_module! {
    r#"#!hebi
      loop: pass
      loop:
        pass
    "#
  }

  check_error! {
    r#"#!hebi
      loop:
      pass
    "#
  }

  check_module! {
    r#"#!hebi
      while true: pass
      while true:
        pass
    "#
  }

  check_error! {
    r#"#!hebi
      while true:
      pass
    "#
  }

  check_module! {
    r#"#!hebi
      for i in iter(): pass
      for i in iter():
        pass
      for i in 0..10: pass
      for i in 0..10:
        pass
      for i in a()..b(): pass
      for i in a()..b():
        pass
      for i in 0..=10: pass
      for i in 0..=10:
        pass
      for i in a()..=b(): pass
      for i in a()..=b():
        pass
    "#
  }

  check_error! {
    r#"#!hebi
      for i in iter():
      pass
    "#
  }

  check_module! {
    r#"#!hebi
      loop:
        loop:
          a
          a
    "#
  }
}

#[test]
fn func_stmt() {
  check_module! {
    r#"#!hebi
      fn f(a, b, c,): pass
      fn f(a, b, c=d): pass
      fn f(a, b=c, d=e,): pass
    "#
  }

  check_error!(r#"fn f(a, b=c, d,): pass"#);
  check_error!(r#"fn f(*,): pass"#);
  check_error!(r#"fn f(**,): pass"#);
  check_error!(r#"fn f(**kwargs, a,): pass"#);
  check_error!(r#"fn f(a, b=,): pass"#);
  check_error!(r#"fn f(a, a): pass"#);
  check_error!(r#"fn f(a, *a): pass"#);
  check_error!(r#"fn f(a, *, a): pass"#);
  check_error!(r#"fn f(a, **a): pass"#);
  check_error! {
    r#"#!hebi
      fn f():
      pass
    "#
  }
  check_error! {
    r#"#!hebi
      fn():
        pass
    "#
  }
}

#[test]
fn ctrl_stmt() {
  check_module! {
    r#"#!hebi
      # simple
      loop:
        break
        continue
      
      fn f():
        yield
        yield v
        return
        return v

      # nested
      loop:
        loop:
          break
          continue
        break
        continue

      fn g():
        fn h():
          yield
          yield v
          return
          return v
        yield
        yield v
        return
        return v
      
      loop:
        fn i():
          yield
          yield v
          return
          return v
        break
        continue

      fn j():
        loop:
          break
          continue
        yield
        yield v
        return
        return v
      
      loop:
        fn k():
          loop:
            break
            continue
          yield
          yield v
          return
          return v
        break
        continue

      fn l():
        loop:
          fn m():
            yield
            yield v
            return
            return v
          break
          continue
        yield
        yield v
        return
        return v
    "#
  }

  check_error! {
    r#"#!hebi
      return v
    "#
  }

  check_error! {
    r#"#!hebi
      yield v
    "#
  }

  check_error! {
    r#"#!hebi
      continue
    "#
  }

  check_error! {
    r#"#!hebi
      break
    "#
  }
}

#[test]
fn print_stmt() {
  check_module! {
    r#"#!hebi
      print "a", 0, true
      print "a", 0, true
    "#
  }

  check_error! {
    r#"#!hebi
      print
        "a"
    "#
  }
}

#[test]
fn class_stmt() {
  check_module! {
    r#"#!hebi
      class T: pass
      class T:
        pass
      class T:
        fn f(v): pass
      class T:
        a = b
        fn f(v): pass
      class T(U): pass
      class T(U):
        pass
      class T(U):
        a = b
      class T(U):
        a = b
        fn f(v): pass
      class T(U):
        a = b
        fn f(v):
          pass
    "#
  }

  check_error! {
    r#"#!hebi
      class
        T: pass
    "#
  }

  check_error! {
    r#"#!hebi
      class T
        : pass
    "#
  }

  check_error! {
    r#"#!hebi
      class T: a = b
    "#
  }

  check_error! {
    r#"#!hebi
      class T: fn f(v): pass
    "#
  }

  check_error! {
    r#"#!hebi
      class T
        (U): pass
    "#
  }

  check_error! {
    r#"#!hebi
      class T(U)
        : pass
    "#
  }

  check_error! {
    r#"#!hebi
      class T(U):
        fn f(v): pass
        a = b
    "#
  }
}

#[test]
fn class_self_and_super() {
  check_module! {
    r#"#!hebi
      class T:
        fn f(self):
          print self
      
      class T(U):
        fn f(self):
          print self, super

      class T(U):
        init(self):
          self.v = super.f()
    "#
  }

  check_error! {
    r#"#!hebi
      self
    "#
  }

  check_error! {
    r#"#!hebi
      class T:
        v = self.f()
    "#
  }

  check_error! {
    r#"#!hebi
      fn f():
        print self
    "#
  }

  check_error! {
    r#"#!hebi
      class T:
        fn f():
          print self
    "#
  }

  check_error! {
    r#"#!hebi
      super
    "#
  }

  check_error! {
    r#"#!hebi
      fn f():
        print super
    "#
  }

  check_error! {
    r#"#!hebi
      class T:
        fn f():
          print self
    "#
  }

  check_error! {
    r#"#!hebi
      class T:
        v = super
    "#
  }

  check_error! {
    r#"#!hebi
      class T:
        fn f():
          print super
    "#
  }

  check_error! {
    r#"#!hebi
      class T(U):
        v = super
    "#
  }

  check_error! {
    r#"#!hebi
      class T(U):
        fn f():
          print super
    "#
  }
}

#[test]
fn duplicate_fields() {
  check_error! {
    r#"#!hebi
      class Test:
        a = 0
        a = 1
    "#
  }
  check_error! {
    r#"#!hebi
      class Test:
        a = 0
        fn a(): pass
    "#
  }
  check_error! {
    r#"#!hebi
      class Test:
        fn a(): pass
        fn a(): pass
    "#
  }
}

#[test]
fn whole_module() {
  check_module! {
    r#"#!hebi
      # variable declaration
      v := 0

      # values
      v = none # none
      v = 0.1 # number
      v = true # bool
      v = "\tas\\df\x2800\n" # string
      v = [none, 0.1, true, "\tas\\df\x2800\n"] # list
      v = {a: none, b: 0.1, c: true, d: "\tas\\df\x2800\n"} # table
      v = {["a"]: none, ["b"]: 0.1, ["c"]: true, ["d"]: "\tas\\df\x2800\n"}
      v = {[0]: none, [1]: 0.1, [2]: true, [3]: "\tas\\df\x2800\n"}

      # operators
      v = 2 + 2
      v = 2 - 2
      v = 2 / 2
      v = 2 * 2
      v = 2 % 2
      v = 2 ** 2
      v = 2 == 2
      v = 2 != 2
      v = 2 > 2
      v = 2 >= 2
      v = 2 < 2
      v = 2 <= 2
      v = -2
      v = !true
      v = true && true
      v = false || true
      v = a ?? b

      # assignment
      v = 1
      v += 1
      v -= 1
      v /= 1
      v *= 1
      v %= 1
      v **= 1
      v ??= 1

      # postfix
      v.a
      v["a"]
      v(a)

      # functions
      fn add(a, b):
        return a + b

      v = add(0, 1)

      fn fact(n):
        if n < 2:
          return n
        else:
          return n * fact(n - 1)

      fn print_fact(n):
        print(fact(n))

      # loops
      # range is an object
      for i in 0..10:
        print(i)

      # `yield` inside `fn` makes it an iterator
      # when called, iterators return an object with a `next` method
      # an iterator is done when its `next` method returns none
      fn counter(start, step, end):
        n := start
        loop:
          yield n
          n += step
          if end && n > end:
            return

      for n in counter(0, 10, 100):
        print(n)

      v = 0
      while v < 10:
        print(v)
        v += 1

      v = 0
      loop:
        if v >= 10:
          break
        print(v)
        v += 1

      if v < 10:
        print("less than 10")
      elif v < 20:
        print("less than 20")
      else:
        print("very large")

      class Test:
        init(self, n):
          self.n = n

        fn get_n(self):
          return self.n

        fn test1(self):
          print("instance", self)

        fn test0():
          print("static", Test)

      v = Test()
      print(v.get_n() == Test.get_n(v)) # true

      v = Test(10)

      Test.test0()
      v.test1()

      # errors
      # no exceptions, panic = abort
      panic("asdf")

      # modules
      import json

      v = json.parse("{\"a\":0, \"b\":1}")
      print(v) # { a: 0, b: 1 }

      # data class, implicit initializer
      class A:
        a = 100
        # init(self, a = 100):
        #   self.a = a

      print(A().a)   # 100
      print(A(10).a) # 10

      class B:
        a = 100
        init(self): # override the implicit initializer
          pass

      print(B().a) # 100
      print(B(10).a) # error

      class C:
        # fields do not have to be declared
        # and may be added in the initializer
        # after `init` is called, the class is frozen
        # no fields/methods may be added or removed
        init(self):
          self.a = 10

      print(C().a) # 10
      C().b = 10 # error: cannot add new field `b` to class `C`

      class A:
        fn inherited(self):
          print("test 0")

      class B(A): pass

      A().inherited() # test 0
      B().inherited() # test 0

      class C(B):
        fn inherited(self): # override
          print("test 1")

      C().inherited() # test 1

      class D(C):
        fn inherited(self): # override with call to super
          super.inherited()
          print("test 2")

      D().inherited() # test 1
                      # test 2

      class X:
       init(self):
          self.v = 10

      class Y(X):
        init(self): # error: `super.init` must be called before accessing `self` or returning in derived constructor
          self.v = 10

      class Z(X):
        init(self, v):
          super.init()
          self.v += v

      print(Z(15).v) # 25
    "#
  }
}

/* #[test]
fn _temp() {
  check_error! {
    r#"#!hebi
      class T: fn f(v): pass
    "#
  }
} */

// semicolons

check_module! {
  many_semicolons,
  r#"#!hebi
    1 + 2; 3 + 4
    1 + 2; 3 + 4;
    print 5; print 6;
    pass; print 7; pass;
    a; b; c; d;
    a + b - c; a / d || c; print abcd;
    print (
      a + b
    ); print (
        c + d
      ); print (
          e + f
        );  
    if true: print x; print y;; print z; if false: print a;; print b
    if true: print x; print y;; elif false: print a;; else: print z; print zz
    if one: print one; if two: print two;; else: print two_else;;;; else: print one_else
  "#
}

check_module! {
  if_statement_supports_semicolons,
  r#"#!hebi
    if true:
      if true: print x; print y
      if true: print z
  "#
}

check_module! {
  for_statement_supports_semicolons,
  r#"#!hebi
    for i in 0..10: print x
    x := 0; for i in 0..10: x += i; print x;; print x;
    "#
}

check_module! {
  for_statements_can_be_nested_on_the_same_line,
  r#"#!hebi
    for y in 0..10: for x in 0..10: print "x * y =", x * y;; print "------------"
  "#
}

check_module! {
  infinite_loop_statements_support_semicolons,
  r#"#!hebi
    loop: print x; print y
    loop: print x;; print y
    loop: print x; print y;; print z
  "#
}

check_module! {
  while_statements_support_semicolons,
  r#"#!hebi
    x := 1; while x % 96 != 0: x += x * 17; x += x * 11;; if x != 46656: print "broken arithmetics"
  "#
}

check_module! {
  import_statements_support_semicolons,
  r#"#!hebi
    import a; import b
    from a import b; import x; from c import d;
    import http; from json import parse; print parse(http.get("https://jsonplaceholder.typicode.com/todos/1"))
  "#
}

check_module! {
  trailing_statements_of_nested_blocks_are_included_in_outer_block_body,
  r#"#!hebi
    while outer: print "outer"; while inner: break;; print "after inner";
  "#
}

check_module! {
  fn_statement_supports_semicolons,
  r#"#!hebi
    fn add(a, b): s := a + b; print "a + b = ", s; return s;; print add(1,2); 
    fn reduce(it, init, func): acc := init; for x in it: acc = func(acc, x);; return acc;; fn reducer(acc, i): return acc + i;; print reduce([1,2,3,4],0, reducer)
  "#
}

check_error! {
  inline_for_indentation_enforcement,
  r#"#!hebi
    for i in 0..10: print x;
      print y

    for y in 0..10: for x in 0..10: print "x * y =", x * y;
        print x, y

    for y in 0..10: for x in 0..10: print "x * y =", x * y;;
      print "---------"
  "#
}

check_error! {
  inline_while_indentation_enforcement,
  r#"#!hebi
    while x % 96 != 0: x += x * 17; x += x * 11;
      if x != 46656: print "broken arithmetics"
  "#
}

check_module! {
  funky_inline_indentation_mix,
  r#"#!hebi
    if cond: print 1;; else:
      print "i == j"

    for i in (call()
    ): for j in (call()
      ): if (
          i 
          != 
          j
        ): print (
            i, 
            j
          ); print (
            j, 
            i
          );; else: 
          print (
          "i == j"
        )

    for one in x(): for two in (
      y()
    ): for three in z(): if (one != two
    ): print one, two;; elif one == two: print (
      "one == two"
                          );; else: 
    print "unreachable"
  "#
}

check_error! {
  scope_no_semi,
  r#"#!hebi
    if true: print x
      print y
  "#
}

check_error! {
  scope_single_semi,
  r#"#!hebi
    if true: print x;
      print y
  "#
}

check_error! {
  scope_double_semi,
  r#"#!hebi
    if true: print x;;
      print y
  "#
}

check_error! {
  scope_nested_no_semi,
  r#"#!hebi
    if true: if true: print x
      print y
  "#
}

check_error! {
  scope_nested_single_semi,
  r#"#!hebi
    if true: if true: print x;
      print y
  "#
}

check_error! {
  scoped_nested_double_semi,
  r#"#!hebi
    if true: if true: print x;;
      print y
  "#
}
