#[macro_export]
macro_rules! check {
  ($name:ident, $input:literal) => {check!($name, __print_func:false, $input);};
  ($name:ident, :print_func $input:literal) => {check!($name, __print_func:true, $input);};
  ($name:ident, __print_func:$print_func:expr, $input:literal) => {
    #[test]
    fn $name() {
      use mu_vm::*;
      use value::object::{Dict, Registry};
      use value::Value;

      let input = indoc::indoc!($input);
      let print_func = $print_func;

      let module = match syntax::parse(input) {
        Ok(module) => module,
        Err(e) => {
          for err in e {
            eprintln!("{}", err.report(input));
          }
          panic!("Failed to parse source, see errors above.")
        }
      };
      let module = match emit::emit(&emit::Context::new(), "[[main]]", &module) {
        Ok(module) => module,
        Err(e) => {
          panic!("failed to emit module:\n{}", e.report(input));
        }
      };
      let module = module.borrow();
      let func = module.main();
      if print_func {
        eprintln!("{}", func.borrow().disassemble(op::disassemble, false));
      }
      let mut vm = Isolate::with_io(Registry::new().into(), Vec::new());
      let value = match vm.call(func.clone().into(), &[], Value::from(Dict::new())) {
        Ok(v) => v,
        Err(e) => {
          panic!("call to func failed with:\n{}", e.report(input));
        }
      };
      let stdout = std::str::from_utf8(vm.io()).unwrap();
      let snapshot =
        format!("# Input:\n{input}\n\n# Result (success):\n{value}\n\n# Stdout:\n{stdout}");
      //insta::assert_snapshot!(snapshot);
    }
  };
}

#[macro_export]
macro_rules! check_error {
  ($name:ident, $input:literal) => {check_error!($name, __print_func:false, $input);};
  ($name:ident, :print_func $input:literal) => {check_error!($name, __print_func:true, $input);};
  ($name:ident, __print_func:$print_func:expr, $input:literal) => {
    #[test]
    fn $name() {
      use mu_vm::*;
      use value::object::{Dict, Registry};
      use value::Value;

      let input = indoc::indoc!($input);
      let print_func = $print_func;

      let module = match syntax::parse(input) {
        Ok(module) => module,
        Err(e) => {
          for err in e {
            eprintln!("{}", err.report(input));
          }
          panic!("Failed to parse source, see errors above.")
        }
      };
      let module = match emit::emit(&emit::Context::new(), "[[main]]", &module) {
        Ok(module) => module,
        Err(e) => {
          panic!("failed to emit module:\n{}", e.report(input));
        }
      };
      let module = module.borrow();
      let func = module.main();
      if print_func {
        eprintln!("{}", func.borrow().disassemble(op::disassemble, false));
      }
      let mut vm = Isolate::with_io(Registry::new().into(), Vec::<u8>::new());
      let error = match vm.call(func.clone().into(), &[], Value::from(Dict::new())) {
        Ok(v) => panic!("call to func succeeded with {v}"),
        Err(e) => e.report(input),
      };
      let stdout = std::str::from_utf8(vm.io()).unwrap();
      let snapshot = format!("# Input:\n{input}\n\n# Result (error):\n{error}\n\n# Stdout:\n{stdout}");
      //insta::assert_snapshot!(snapshot);
    }
  };
}
