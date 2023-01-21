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
    let chunk = match emit("test", &module) {
      Ok(chunk) => chunk,
      Err(e) => {
        panic!("failed to emit chunk:\n{}", e.report(input));
      }
    };
    let snapshot = format!("# Input:\n{input}\n\n# Chunk:\n{}", chunk.disassemble());
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
