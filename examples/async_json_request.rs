#[cfg(not(feature = "serde"))]
fn main() {}

#[cfg(feature = "serde")]
#[tokio::main]
async fn main() {
  use hebi::{Hebi, NativeModule, Result, Scope, Str, Value, ValueDeserializer};
  use serde::de::DeserializeSeed;

  let client = reqwest::Client::default();

  async fn get(scope: Scope<'_>, client: reqwest::Client) -> Result<Value<'_>> {
    let url = scope.param::<Str>(0)?;
    let request = client.get(url.as_str());
    let response = request.send().await.map_err(hebi::Error::user)?;
    let bytes = response.bytes().await.map_err(hebi::Error::user)?.to_vec();
    let str = String::from_utf8(bytes).map_err(hebi::Error::user)?;
    let value = ValueDeserializer::new(scope.global())
      .deserialize(&mut serde_json::de::Deserializer::from_str(&str))
      .map_err(hebi::Error::user)?;
    Ok(value)
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

response := http.get("https://jsonplaceholder.typicode.com/todos/1")
print "title: ", response["title"], " id: ", response["userId"]
response
"#,
    )
    .await
    .unwrap();

  println!("Result is:\n{result:?}");
  println!("Serialized: {}", serde_json::to_string(&result).unwrap());
}
