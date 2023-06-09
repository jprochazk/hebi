use std::time::Duration;

use hebi::prelude::*;

#[tokio::main]
async fn main() {
  async fn example(sender: flume::Sender<String>) -> i32 {
    tokio::time::sleep(Duration::from_millis(10)).await;

    sender.send_async("test".into()).await.unwrap();

    10i32
  }

  let (tx, rx) = flume::bounded(256);

  let module = NativeModule::builder("test")
    .async_function("example", move |_| example(tx.clone()))
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
  println!("Channel got: {}", rx.recv_async().await.unwrap());
}
