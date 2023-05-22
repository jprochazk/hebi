use hebi::{Hebi, NativeModule, Scope, Value};

fn main() {
  fn print_value(scope: Scope) -> hebi::Result<()> {
    let value = scope.param::<Value>(0)?;
    scope.global().println(format_args!("value is: {value}"))?;
    Ok(())
  }

  let module = NativeModule::builder("example")
    .function("print_value", print_value)
    .finish();

  let mut hebi = Hebi::builder().output(Vec::<u8>::new()).finish();
  hebi.register(&module);
  hebi
    .eval(
      r#"
import example
example.print_value(100)
"#,
    )
    .unwrap();

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
  print!("{output}");
}
