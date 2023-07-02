use hebi::prelude::*;

fn main() {
  fn example(_: Scope) -> i32 {
    100i32
  }

  fn add1(scope: Scope) -> hebi::Result<i32> {
    let value = scope.param::<i32>(0)?;
    Ok(value + 1)
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

  println!("Result is: {result}");
}
