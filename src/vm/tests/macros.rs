macro_rules! check {
  ($name:ident, $input:literal) => {
    #[test]
    #[allow(non_snake_case)]
    fn $name() {
      let input = indoc::indoc!($input);
      let hebi = Hebi::new();
      let snapshot = format!("{:#?}", hebi.eval(input));
      assert_snapshot!(snapshot);
    }
  };
  (module $name:ident, { $($module:ident: $code:literal),* }, $input:literal) => {
    #[test]
    #[allow(non_snake_case)]
    fn $name() {
      let input = indoc::indoc!($input);
      let hebi = Hebi::new();
      hebi.set_module_loader(
        TestModuleLoader::new(&[$(
          (stringify!($module), indoc::indoc!($code))
        ),*])
      );
      let snapshot = format!("{:#?}", hebi.eval(input));
      assert_snapshot!(snapshot);
    }
  };
}
