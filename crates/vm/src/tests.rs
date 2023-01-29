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
    let func = func.as_func().unwrap().disassemble(op::disassemble, false);
    let snapshot = format!("# Input:\n{input}\n\n# Func:\n{func}\n\n# Result:\n{value}");
    insta::assert_snapshot!(snapshot);
  }};
}

#[test]
fn simple() {
  check!(r#"1 + 1"#);
}
