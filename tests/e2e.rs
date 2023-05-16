use hebi::module::NativeModule;
use hebi::{Hebi, IntoValue, Result, Scope, Value};

#[test]
fn hebi_e2e() {
  fn example<'cx>(scope: &'cx Scope<'cx>) -> Result<Value<'cx>> {
    let value = 100i32;
    value.into_value(scope)
  }

  fn add1<'cx>(scope: &'cx Scope<'cx>) -> Result<Value<'cx>> {
    let value = scope
      .argument(0)
      .ok_or_else(|| hebi::error!("Missing argument 0"))?;
    let value = value
      .as_int()
      .ok_or_else(|| hebi::error!("First argument must be an integer"))?;

    let value = value + 1;

    value.into_value(scope)
  }

  let module = NativeModule::builder("test")
    .function("example", example)
    .function("add1", add1)
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  let result = hebi
    .eval(
      r#"
from test import example, add1
add1(example())
"#,
    )
    .unwrap();

  assert_eq!(result.as_int(), Some(101));
}
