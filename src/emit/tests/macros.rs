macro_rules! check {
  ($name:ident, $(as_module=$as_module:expr,)? $input:literal) => {
      #[allow(unused_mut, unused_assignments)]
      #[test]
      fn $name() {
      let mut as_module = false;
      $(as_module = $as_module;)?
      let global = $crate::vm::global::Global::default();
      let input = indoc::indoc!($input);
      let module = match syntax::parse(global.clone(), input) {
        Ok(module) => module,
        Err(e) => {
          for err in e.errors() {
            eprintln!("{}", err.report(input, true));
          }
          panic!("Failed to parse source, see errors above.")
        }
      };
      let module = emit(global, &module, "main", !as_module);
      let snapshot = format!(
        "# Input:\n{input}\n\n# Func:\n{}\n\n",
        module.root.disassemble(),
      );
      assert_snapshot!(snapshot);
    }
  };
}
