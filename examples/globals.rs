use hebi::{Hebi, IntoValue, NativeModule, Result, Scope};

fn main() {
  fn example(scope: Scope) -> Result<()> {
    scope
      .globals()
      .set(scope.new_string("in_native_fn"), scope.param(0)?);
    Ok(())
  }

  let module = NativeModule::builder("test")
    .function("example", example)
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  hebi.globals().set(
    hebi.new_string("external"),
    (100i32).into_value(hebi.global()).unwrap(),
  );

  let scope = hebi.scope();
  hebi.globals().set(
    scope.new_string("value"),
    scope.new_string("test").into_value(scope.global()).unwrap(),
  );

  let result = hebi
    .eval(
      r#"
from test import example
example(external)
in_native_fn
"#,
    )
    .unwrap();

  println!("Result is: {result}");
}
