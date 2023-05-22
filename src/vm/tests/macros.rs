macro_rules! check {
  ($name:ident, $input:literal) => {
    #[tokio::test]
    #[allow(non_snake_case)]
    async fn $name() {
      let input = indoc::indoc!($input);
      let mut hebi = crate::Hebi::default();
      let snapshot = format!("{:#?}", hebi.eval_async(input).await);
      assert_snapshot!(snapshot);
    }
  };
  (module $name:ident, { $($module:ident: $code:literal),* }, $input:literal) => {
    #[tokio::test]
    #[allow(non_snake_case)]
    async fn $name() {
      let input = indoc::indoc!($input);
      let mut hebi = crate::Hebi::builder().module_loader(
        TestModuleLoader::new(&[
          $((stringify!($module), indoc::indoc!($code))),*
        ])
      ).finish();
      let snapshot = format!("{:#?}", hebi.eval_async(input).await);
      assert_snapshot!(snapshot);
    }
  };
}
