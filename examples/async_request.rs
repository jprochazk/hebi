use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use hebi::prelude::*;

#[tokio::main]
async fn main() {
  let client = reqwest::Client::default();

  async fn get(scope: Scope<'_>, client: reqwest::Client) -> hebi::Result<Str<'_>> {
    let url = scope.param::<Str>(0)?;
    let request = client.get(url.as_str());
    let response = request.send().await.map_err(hebi::Error::user)?;
    let bytes = response.bytes().await.map_err(hebi::Error::user)?.to_vec();
    let str = String::from_utf8(bytes).map_err(hebi::Error::user)?;
    Ok(scope.new_string(str))
  }

  let module = NativeModule::builder("http")
    .async_function("get", move |scope| get(scope, client.clone()))
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  let source = r#"
from http import get
get("https://jsonplaceholder.typicode.com/todos/1")
  "#;

  let result = AssertUnwindSafe(hebi.eval_async(source))
    .catch_unwind()
    .await;

  match result {
    Ok(result) => match result {
      Ok(value) => println!("Result is:\n{value}"),
      Err(e) => {
        eprintln!("{}", e.report(source, true))
      }
    },
    Err(panic) => {
      println!("hebi panicked");
      for (key, value) in hebi.global().entries() {
        println!("{key}: {value}")
      }
      std::panic::panic_any(panic)
    }
  }
}
