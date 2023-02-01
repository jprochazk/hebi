use super::*;

macro_rules! check {
  ($input:literal) => {check!(__print_func:false, $input)};
  (:print_func $input:literal) => {check!(__print_func:true, $input)};
  (__print_func:$print_func:expr, $input:literal) => {{
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
    let func = match emit::emit(&emit::Context::new(), "[[main]]", &module) {
      Ok(func) => func,
      Err(e) => {
        panic!("failed to emit func:\n{}", e.report(input));
      }
    };
    if $print_func {
      eprintln!("{}", func.as_func().unwrap().disassemble(op::disassemble, false));
    }
    let mut vm = Isolate::with_io(Vec::new());
    let value = match vm.call(func.clone(), &[], Value::from(Dict::new())) {
      Ok(v) => v,
      Err(e) => {
        panic!("call to func failed with:\n{}", e.report(input));
      }
    };
    let stdout = std::str::from_utf8(vm.io()).unwrap();
    let snapshot = format!("# Input:\n{input}\n\n# Result (success):\n{value}\n\n# Stdout:\n{stdout}");
    insta::assert_snapshot!(snapshot);
  }};
}

macro_rules! check_error {
  ($input:literal) => {check_error!(__print_func:false, $input)};
  (:print_func $input:literal) => {check_error!(__print_func:true, $input)};
  (__print_func:$print_func:expr, $input:literal) => {{
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
    let func = match emit::emit(&emit::Context::new(), "[[main]]", &module) {
      Ok(func) => func,
      Err(e) => {
        panic!("failed to emit func:\n{}", e.report(input));
      }
    };
    if $print_func {
      eprintln!("{}", func.as_func().unwrap().disassemble(op::disassemble, false));
    }
    let mut vm = Isolate::with_io(Vec::<u8>::new());
    let error = match vm.call(func.clone(), &[], Value::from(Dict::new())) {
      Ok(v) => panic!("call to func succeeded with {v}"),
      Err(e) => e.report(input),
    };
    let stdout = std::str::from_utf8(vm.io()).unwrap();
    let snapshot = format!("# Input:\n{input}\n\n# Result (error):\n{error}\n\n# Stdout:\n{stdout}");
    insta::assert_snapshot!(snapshot);
  }};
}

#[test]
fn literals() {
  check!(r#"none"#);
  check!(r#"true"#);
  check!(r#"false"#);
  check!(r#"1"#);
  check!(r#"0.1"#);
  check!(r#"1.5e3"#);
  check!(r#"3.14e-3"#);
  check!(r#""\tas\\df\u{2800}\x28\n""#);
  check!(r#"[0, 1, 2]"#);
  check!(r#"{a:0, b:1, c:2}"#);
  check!(r#"{["a"]:0, ["b"]:1, ["c"]:2}"#);
}

#[test]
fn simple() {
  check!(r#"2 + 2"#);
  check!(r#"2 - 2"#);
  check!(r#"2 / 2"#);
  check!(r#"2 ** 2"#);
  check!(r#"2 * 2"#);
  check!(r#"2 % 2"#);
  check!(r#"2 == 2"#);
  check!(r#"2 != 2"#);
  check!(r#"2 > 2"#);
  check!(r#"2 >= 2"#);
  check!(r#"2 < 2"#);
  check!(r#"2 <= 2"#);
  check!(r#"true && false"#);
  check!(r#"true || false"#);
  check!(r#"none ?? 2"#);
  check!(r#"2 ?? none"#);
  check!(r#"+2"#);
  check!(r#"-2"#);
  check!(r#"!true"#);
  check!(r#"!false"#);
}

#[test]
fn precedence() {
  check!(r#"3 * 2 + 1"#);
  check!(r#"1 + 2 * 3"#);
}

#[test]
fn assignment() {
  check! {
    r#"
      v := 10
      v = 5
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v += 2
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v -= 2
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v *= 2
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v /= 2
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v **= 2
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v %= 2
      print v
    "#
  }
  check! {
    r#"
      v := none
      v ??= 2
      print v
    "#
  }
  check! {
    r#"
      v := 10
      v ??= 2
      print v
    "#
  }
}

#[test]
fn object_load_named() {
  check_error! {
    r#"
      v := {}
      print v.a
    "#
  }
  check! {
    r#"
      v := { a: 10 }
      print v.a
    "#
  }
  check! {
    r#"
      v := {}
      print ?v.a
    "#
  }
  check_error! {
    r#"
      v := {}
      print ?v.a
      print v.a
    "#
  }
  check! {
    r#"
      v := { a: { b: 10 } }
      print ?v.a.b
    "#
  }
  check_error! {
    r#"
      v := { a: {} }
      print v.a.b
    "#
  }
  check! {
    r#"
      v := { a: {} }
      print ?v.a.b
    "#
  }
}

#[test]
fn object_load_keyed() {
  check_error! {
    r#"
      v := {}
      print v["a"]
    "#
  }
  check! {
    r#"
      v := { a: 10 }
      print v["a"]
    "#
  }
  check! {
    r#"
      v := {}
      print ?v["a"]
    "#
  }
  check_error! {
    r#"
      v := {}
      print ?v["a"]
      print v["a"]
    "#
  }
  check! {
    r#"
      v := { a: { b: 10 } }
      print ?v["a"]["b"]
    "#
  }
  check_error! {
    r#"
      v := { a: {} }
      print v["a"]["b"]
    "#
  }
  check! {
    r#"
      v := { a: {} }
      print ?v["a"]["b"]
    "#
  }
}

#[test]
fn branch() {
  check! {
    r#"
      v := 10
      if v > 5:
        print "yes"
      else:
        print "no"
    "#
  }
}

#[test]
fn loops() {
  check! {
    r#"
      i := 0
      loop:
        if i >= 10:
          break
        print i
        i += 1
    "#
  }
  check! {
    r#"
      i := 0
      while i < 10:
        print i
        i += 1
    "#
  }
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
      for i in 10..0:
        print i
    "#
  }
  check! {
    r#"
      start := 0
      end := 10
      for i in start..end:
        print i
    "#
  }
  check! {
    r#"
      start := 0
      end := 10
      for i in start..=end:
        print i
    "#
  }
  check! {
    r#"
      for i in 0..10:
        if i % 2 == 0: continue
        print i
    "#
  }
}

#[test]
fn fn_call() {
  check! {
    r#"
      fn f():
        print "test"

      f()
      f()
    "#
  }
  check! {
    r#"
      fn f(a, b, c):
        print a, b, c

      f(0, 1, 2)
      f(0, 1, 2)
    "#
  }
  check_error! {
    r#"
      fn f(a, b, c):
        print a, b, c

      f()
    "#
  }
  check! {
    r#"
      fn f(a, *rest):
        print a, rest

      f(0)
      f(0, 1, 2)
    "#
  }
}

#[test]
fn fn_call_default() {
  check! {
    r#"
      fn f(a=10):
        print a
      
      f()
      f(1)
    "#
  }
  check! {
    r#"
      fn f(a, b=10):
        print a, b
      
      f(1)
      f(1,2)
    "#
  }
  check_error! {
    r#"
      fn f(a=10):
        print a
      
      f(1,2)
    "#
  }
  check_error! {
    r#"
      fn f(a, b=10):
        print a
      
      f(1,2,3)
    "#
  }
  check_error! {
    r#"
      fn f(a, b=10):
        print a
      
      f()
    "#
  }
  check! {
    r#"
      fn f(a, b=10, *c):
        print a, b, c
      
      f(1)
      f(1,2)
      f(1,2,3)
    "#
  }
}

#[test]
fn fn_call_kw() {
  check! {
    r#"
      fn f(*, a):
        print a

      f(a=10)
    "#
  }
  check! {
    r#"
      fn f(*, a, b):
        print a, b

      f(a=1, b=2)
      f(b=2, a=1)
    "#
  }
  check! {
    r#"
      fn f(*, a, b=10):
        print a, b

      f(a=1)
      f(a=1, b=2)
    "#
  }
  check! {
    r#"
      fn f(*, a, b, **kw):
        print a, b, kw

      f(a=1, b=2, c=3)
      f(c=3, b=2, a=1)
    "#
  }
  check! {
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
    r#"
      fn f(*, a):
        print a

      f()
    "#
  }
  check_error! {
    r#"
      fn f(*, a, b):
        print a, b

      f()
    "#
  }
  check_error! {
    r#"
      fn f(*, a, b, **kw):
        print a, b, kw

      f()
    "#
  }
  check_error! {
    r#"
      fn f(*, a, b, **kw):
        print a, b, kw

      f(c=10)
    "#
  }
}

#[test]
fn call_closure() {
  check! {
    r#"
      fn a():
        v := 10
        fn b():
          print v
        return b
  
      a()()
    "#
  }
  check! {
    r#"
      fn counter(start=0, *, step=1):
        state := { value: start }
        fn inner():
          temp := state.value
          state.value += step
          return temp
        return inner
      
      c := counter()
      print c()
      print c()
      print c()
    "#
  }
  check! {
    r#"
      fn a():
        fn b():
          v := 10
          fn c():
            fn d():
              return v
            return d
          return c
        return b
      
      print a()()()()
    "#
  }
}
