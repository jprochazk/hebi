use hebi::{Hebi, NativeModule, Result, Scope, Str};

#[tokio::main(flavor = "current_thread")]
async fn main() {
  let client = reqwest::Client::default();

  async fn get(scope: Scope<'_>, client: reqwest::Client) -> Result<Str<'_>> {
    let url = scope.param::<Str>(0)?;
    let response = client
      .get(url.as_str())
      .send()
      .await
      .map_err(hebi::Error::user)?;
    let bytes = response.bytes().await.map_err(hebi::Error::user)?;
    let data = bytes.to_vec();
    let str = String::from_utf8(data).map_err(hebi::Error::user)?;
    Ok(scope.new_string(str))
  }

  let module = NativeModule::builder("http")
    .async_function("get", move |scope| get(scope, client.clone()))
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  let result = hebi
    .eval_async(
      r#"
import http

http.get("https://jsonplaceholder.typicode.com/todos/1")
"#,
    )
    .await
    .unwrap();

  println!("Result is:\n{result}");
}
