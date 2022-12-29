use diag::Source;
use indoc::indoc;
use peg::error::ParseError;
use span::Span;

use super::*;

fn report(source: &str, err: ParseError<Span>) -> String {
  let message = if let Some(err) = err.expected.tokens().find(|&t| t.starts_with("@@")) {
    err.strip_prefix("@@").unwrap_or(err).to_string()
  } else {
    err.expected.to_string()
  };

  diag::Report::error()
    .source(Source::string(source))
    .message(message)
    .span(err.location)
    .build()
    .unwrap()
    .emit_to_string()
    .unwrap()
}

fn print_tokens(lex: &Lexer) {
  for token in lex.debug_tokens() {
    println!("{token:?}");
  }
  // let tokens = lex.debug_tokens().collect::<Vec<_>>();
  // insta::assert_debug_snapshot!(tokens);
}

macro_rules! check_module {
  ($input:literal) => {check_module!(__inner $input, false)};
  (? $input:literal) => {check_module!(__inner $input, true)};
  (__inner $input:literal , $print_tokens:expr) => {{
    let input = indoc!($input);
    let lex = Lexer::lex(input).unwrap();
    if $print_tokens { print_tokens(&lex); }
    match parse(&lex) {
      Ok(module) => insta::assert_debug_snapshot!(module),
      Err(e) => {
        eprintln!("{}", report(input, e));
        panic!("Failed to parse source, see errors above.")
      }
    };
  }};
}

macro_rules! check_expr {
  ($input:literal) => {check_expr!(__inner $input, false)};
  (? $input:literal) => {check_expr!(__inner $input, true)};
  (__inner $input:literal , $print_tokens:expr) => {{
    let input = indoc!($input);
    let lex = Lexer::lex(input).unwrap();
    if $print_tokens { print_tokens(&lex); }
    match grammar::expr(&lex, &StateRef::new(&lex)) {
      Ok(module) => insta::assert_debug_snapshot!(module),
      Err(e) => {
        eprintln!("{}", report(input, e));
        panic!("Failed to parse source, see errors above.")
      }
    };
  }};
}

macro_rules! check_error {
  ($input:literal) => {check_error!(__inner $input, false)};
  (? $input:literal) => {check_error!(__inner $input, true)};
  (__inner $input:literal , $print_tokens:expr) => {{
    let input = indoc!($input);
    let lex = Lexer::lex(input).unwrap();
    if $print_tokens { print_tokens(&lex); }
    match parse(&lex) {
      Ok(_) => panic!("module parsed successfully"),
      Err(e) => insta::assert_snapshot!(report(input, e)),
    };
  }};
}

#[test]
fn test_import_path() {
  check_module! {
    r#"
      use a
      use a.b
      use a.b.c
      use a.{b, c}
      use a.{b.{c}, d.{e}}
      use {a.{b}, c.{d}}
      use {a, b, c,}
    "#
  };

  check_module! {
    r#"
      use a as x
      use a.b as x
      use a.b.c as x
      use a.{b as x, c as y}
      use a.{b.{c as x}, d.{e as y}}
      use {a.{b as x}, c.{d as y}}
      use {a as x, b as y, c as z,}
    "#
  };

  check_error! {
    r#"
      use a
        use b
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
  check_expr!(r#"a && b"#);
  check_expr!(r#"a || b"#);
  check_expr!(r#"a ?? b"#);

  check_module! {
    r#"
      a + b
      c + d
    "#
  };

  check_error! {
    r#"
      a +
        b
    "#
  }

  check_error! {
    r#"
      a
      + b
    "#
  }
}

#[test]
fn unary_expr() {
  // check_expr!(r#"+a"#);
  check_expr!(r#"-a"#);
  check_expr!(r#"!a"#);
}

#[test]
fn postfix_expr() {
  check_expr!(r#"a.b[c].d"#);
  check_module! {
    r#"
      a.b[c].d
      a.b[c].d
    "#
  };

  check_error! {
    r#"
      a
      .b[c].d
    "#
  }
  check_error! {
    r#"
      a.b[c]
      .d
    "#
  }
}

#[test]
fn call_expr() {
  check_expr!(r#"a(b, c, d=e, f=g)"#);
  check_module! {
    r#"
      a(b, c, d=e, f=g)
      a(
        b, 
      c, d
          =e, 
        f=
        g,
          )
    "#
  };

  check_error! {
    r#"
      a(b=c, d)
    "#
  }
}

#[test]
fn simple_literal_expr() {
  check_module! {
    r#"
      null
      true
      false
      1
      0.1
      1.5e3
      3.14e-3
      "\tas\\df\x2800\n"
    "#
  }
}

#[test]
fn array_literal_expr() {
  check_module! {
    r#"
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
fn object_literal_expr() {
  check_module! {
    r#"
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
    r#"
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
    r#"
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
    r#"
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
    r#"
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
    r#"
      a
        = b
    "#
  }
  check_error! {
    r#"
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
fn loop_stmts() {
  check_module! {
    r#"
      loop: pass
      loop:
        pass
    "#
  }

  check_error! {
    r#"
      loop:
      pass
    "#
  }

  check_module! {
    r#"
      while true: pass
      while true:
        pass
    "#
  }

  check_error! {
    r#"
      while true:
      pass
    "#
  }

  check_module! {
    r#"
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
    r#"
      for i in iter():
      pass
    "#
  }
}

#[test]
fn func_stmt() {
  check_module! {
    r#"
      fn f(): pass
      fn f():
        pass
      fn f(a): pass
      fn f(a,): pass
      fn f(a, b): pass
      fn f(a, b,): pass
    "#
  }

  check_error! {
    r#"
      fn f():
      pass
    "#
  }
}

#[test]
fn ctrl_stmt() {
  check_module! {
    r#"
      break
      continue
      return v
      yield v
    "#
  }
}

#[test]
fn class_stmt() {
  check_module! {
    r#"
      class T: pass
      class T:
        pass
      class T:
        f(v): pass
      class T:
        a = b
        f(v): pass
      class T(U): pass
      class T(U):
        pass
      class T(U):
        a = b
      class T(U):
        a = b
        f(v): pass
    "#
  }
}

#[test]
fn whole_module() {
  /* check_module! {?
    r#"
      loop:
        loop:
          a
          a
    "#
  } */

  check_module! {
    r#"
      # variable declaration
      v := 0

      # values
      v = null # null
      v = 0.1 # number
      v = true # bool
      v = "\tas\\df\x2800\n" # string
      v = [null, 0.1, true, "\tas\\df\x2800\n"] # array
      # object
      v = {a: null, b: 0.1, c: true, d: "\tas\\df\x2800\n"}
      v = {["a"]: null, ["b"]: 0.1, ["c"]: true, ["d"]: "\tas\\df\x2800\n"}
      v = {[0]: null, [1]: 0.1, [2]: true, [3]: "\tas\\df\x2800\n"}

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

      fn fib(n):
        if n < 2:
          return n
        else:
          return n * fib(n - 1)

      fn print_fib(n):
        print(fib(n))

      # loops
      # range is an object
      for i in 0..10:
        print(i)

      # `yield` inside `fn` makes it an iterator
      # when called, iterators return an object with a `next` method
      # an iterator is done when its `next` method returns null
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

        get_n(self):
          return self.n

        test1(self):
          print("instance", self)

        test0():
          print("static", Test)

      v = Test()
      print(v.get_n() == Test.get_n(v)) # true

      v = Test(n=10)

      Test.test0()
      v.test1()

      # errors
      # no exceptions, panic = abort
      panic("asdf")

      # modules
      # json_test.t
      use json
      # other ways to import:
      # use json.parse
      # use json.{parse}
      # use {json}
      # use {json.parse}
      # use {json.{parse}}

      v = json.parse("{\"a\":0, \"b\":1}")
      print(v) # { a: 0, b: 1 }

      # data class, implicit initializer
      class A:
        a = 100
        # init(self, a = 100):
        #   self.a = a

      print(A().a)     # 100
      print(A(a=10).a) # 10

      class B:
        a = 100
        init(self): # override the implicit initializer
          pass

      print(B().a)   # 100
      # `a` is ignored
      print(B(a=10)) # 100

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
        inherited(self):
          print("test 0")

      class B(A): pass

      A().inherited() # test 0
      B().inherited() # test 0

      class C(B):
        inherited(self): # override
          print("test 1")

      C().inherited() # test 1

      class D(C):
        inherited(self): # override with call to super
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
          super.init(self)
          self.v += v

      print(Z(v=15).v) # 25
    "#
  }
}
