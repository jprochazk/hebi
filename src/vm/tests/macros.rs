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
}
