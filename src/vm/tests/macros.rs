macro_rules! check {
  ($name:ident, $input:literal) => {
    #[tokio::test]
    #[allow(non_snake_case)]
    async fn $name() {
      let input = indoc::indoc!($input);
      let mut hebi = crate::vm::Vm::new();
      let snapshot = format!("{:#?}", hebi.eval(input).await);
      assert_snapshot!(snapshot);
    }
  };
  (module $name:ident, { $($module:ident: $code:literal),* }, $input:literal) => {
    #[tokio::test]
    #[allow(non_snake_case)]
    async fn $name() {
      let input = indoc::indoc!($input);
      let mut hebi = crate::vm::Vm::with_module_loader(
        TestModuleLoader::new(&[
          $((stringify!($module), indoc::indoc!($code))),*
        ])
      );
      let snapshot = format!("{:#?}", hebi.eval(input).await);
      assert_snapshot!(snapshot);
    }
  };
}
