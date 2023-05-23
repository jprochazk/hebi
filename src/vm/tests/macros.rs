macro_rules! check {
  ($name:ident, $source:literal) => {
    #[tokio::test]
    #[allow(non_snake_case)]
    async fn $name() {
      let source = indoc::indoc!($source);
      let mut hebi = crate::Hebi::builder().output(Vec::<u8>::new()).finish();
      let result = format!("{:#?}", hebi.eval_async(source).await);
      let output = String::from_utf8(
        hebi
          .global()
          .output()
          .as_any()
          .downcast_ref::<Vec<u8>>()
          .cloned()
          .unwrap(),
      )
      .unwrap();
      let snapshot = if output.is_empty() {
        format!("# Source:\n{source}\n\n# Result:\n{result}")
      } else {
        format!("# Source:\n{source}\n\n# Result:\n{result}\n\n# Output:\n{output}")
      };
      assert_snapshot!(snapshot);
    }
  };
  (module $name:ident, { $($module:ident: $code:literal),* }, $source:literal) => {
    #[tokio::test]
    #[allow(non_snake_case)]
    async fn $name() {
      let source = indoc::indoc!($source);
      let mut hebi = crate::Hebi::builder()
        .output(Vec::<u8>::new())
        .module_loader(
          TestModuleLoader::new(&[
            $((stringify!($module), indoc::indoc!($code))),*
          ])
        )
        .finish();
      let result = format!("{:#?}", hebi.eval_async(source).await);
      let output = String::from_utf8(
        hebi
          .global()
          .output()
          .as_any()
          .downcast_ref::<Vec<u8>>()
          .cloned()
          .unwrap(),
      )
      .unwrap();
      let snapshot = if output.is_empty() {
        format!("# Source:\n{source}\n\n# Result:\n{result}")
      } else {
        format!("# Source:\n{source}\n\n# Result:\n{result}\n\n# Output:\n{output}")
      };
      assert_snapshot!(snapshot);
    }
  };
}
