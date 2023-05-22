use hebi::{Hebi, IntoValue, NativeModule, Result, Scope};

fn main() {
  fn example(scope: Scope) -> Result<()> {
    scope
      .global()
      .set(scope.new_string("internal"), scope.param(0)?);
    Ok(())
  }

  let module = NativeModule::builder("test")
    .function("example", example)
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  hebi.global().set(
    hebi.new_string("external"),
    (100i32).into_value(hebi.global()).unwrap(),
  );

  hebi
    .eval(
      r#"
from test import example

example(50)

print "external: ", external
print "internal: ", internal
"#,
    )
    .unwrap();
}
