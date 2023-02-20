#[macro_export]
macro_rules! check {
  ($name:ident, $input:literal) => {
    #[test]
    fn $name() {
      use mu_runtime::value::object::{Dict, Registry};
      use mu_runtime::*;

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
      let module = match emit::emit(&emit::Context::new(), "test_func", &module) {
        Ok(module) => module,
        Err(e) => {
          panic!("failed to emit module:\n{}", e.report(input));
        }
      };
      let func = module.main();
      eprintln!("{}", func.disassemble(false));
      let mut vm = Isolate::with_io(Handle::alloc(Registry::new()), Vec::new());
      let value = match vm.call(
        Value::object(func),
        &[],
        Value::object(Handle::alloc(Dict::new())),
      ) {
        Ok(v) => v,
        Err(e) => {
          panic!(
            "call to func failed with:\n{}",
            e.stack_trace(Some(input.into()))
          );
        }
      };
      let stdout = std::str::from_utf8(vm.io()).unwrap();
      let snapshot =
        format!("# Input:\n{input}\n\n# Result (success):\n{value}\n\n# Stdout:\n{stdout}");
      insta::assert_snapshot!(snapshot);
    }
  };
}

#[macro_export]
macro_rules! check_error {
  ($name:ident, $input:literal) => {
    #[test]
    fn $name() {
      use mu_runtime::value::object::{Dict, Registry};
      use mu_runtime::*;

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
      let module = match emit::emit(&emit::Context::new(), "test_func", &module) {
        Ok(module) => module,
        Err(e) => {
          panic!("failed to emit module:\n{}", e.report(input));
        }
      };
      let func = module.main();
      eprintln!("{}", func.disassemble(false));
      let mut vm = Isolate::with_io(Handle::alloc(Registry::new()), Vec::<u8>::new());
      let error = match vm.call(
        Value::object(func),
        &[],
        Value::object(Handle::alloc(Dict::new())),
      ) {
        Ok(v) => panic!("call to func succeeded with {v}"),
        Err(e) => e.stack_trace(Some(input.into())),
      };
      let stdout = std::str::from_utf8(vm.io()).unwrap();
      let snapshot =
        format!("# Input:\n{input}\n\n# Result (error):\n{error}\n\n# Stdout:\n{stdout}");
      insta::assert_snapshot!(snapshot);
    }
  };
}
