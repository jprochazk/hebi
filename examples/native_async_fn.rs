use std::time::Duration;

use hebi::prelude::*;

#[tokio::main]
async fn main() {
  async fn example(_: Scope<'_>) -> i32 {
    tokio::time::sleep(Duration::from_millis(10)).await;

    10i32
  }

  let module = NativeModule::builder("test")
    .async_function("example", example)
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  let result = hebi
    .eval_async(
      r#"
from test import example
example()
  "#,
    )
    .await
    .unwrap();

  println!("Result is: {result}");
}
