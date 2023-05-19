use hebi::{Hebi, NativeModule, Result, Scope, Str};

#[tokio::main]
async fn main() {
  let client = reqwest::Client::default();

  async fn get(scope: Scope<'_>, client: reqwest::Client) -> Result<Str<'_>> {
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

  let result = hebi
    .eval_async(
      r#"
from http import get
get("https://jsonplaceholder.typicode.com/todos/1")
"#,
    )
    .await
    .unwrap();

  println!("Result is:\n{result}");
}
