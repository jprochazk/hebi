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
    let func = match emit::emit(&emit::Context::new(), "[[main]]", &module) {
      Ok(func) => func,
      Err(e) => {
        panic!("failed to emit func:\n{}", e.report(input));
      }
    };
    let mut vm = Isolate::with_io(Vec::new());
    let value = match vm.call(func.clone(), &[], Default::default()) {
      Ok(v) => v,
      Err(e) => {
        panic!("call to func failed with:\n{}", e.report(input));
      }
    };
    let stdout = std::str::from_utf8(&vm.get_io()[..]).unwrap();
    let func = func.as_func().unwrap().disassemble(op::disassemble, false);
    let snapshot = format!("# Input:\n{input}\n\n# Func:\n{func}\n\n# Result (success):\n{value}\n\n# Stdout:\n{stdout}");
    insta::assert_snapshot!(snapshot);
  }};
}

macro_rules! check_error {
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
    let func = match emit::emit(&emit::Context::new(), "[[main]]", &module) {
      Ok(func) => func,
      Err(e) => {
        panic!("failed to emit func:\n{}", e.report(input));
      }
    };
    let mut vm = Isolate::with_io(Vec::<u8>::new());
    let error = match vm.call(func.clone(), &[], Default::default()) {
      Ok(v) => panic!("call to func succeeded with {v}"),
      Err(e) => e.report(input),
    };
    let stdout = std::str::from_utf8(&vm.get_io()[..]).unwrap();
    let func = func.as_func().unwrap().disassemble(op::disassemble, false);
    let snapshot = format!(
      "# Input:\n{input}\n\n# Func:\n{func}\n\n# Result (error):\n{error}\n\n# Stdout:\n{stdout}"
    );
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
