use hebi::{Hebi, NativeModule, Scope, Str, This};

#[tokio::main]
async fn main() {
  struct Client {
    inner: reqwest::Client,
  }

  impl Client {
    fn new() -> Self {
      Self {
        inner: reqwest::Client::new(),
      }
    }

    async fn get(scope: Scope<'_>, this: This<'_, Client>) -> hebi::Result<String> {
      let url = scope.param::<Str>(0)?;
      let request = this.inner.get(url.as_str());
      let response = request.send().await.map_err(hebi::Error::user)?;
      let bytes = response.bytes().await.map_err(hebi::Error::user)?.to_vec();
      let str = String::from_utf8(bytes).map_err(hebi::Error::user)?;
      Ok(str)
    }
  }

  let module = NativeModule::builder("http")
    .class::<Client>("Client", |class| {
      class
        .init(|_| Ok(Client::new()))
        .async_method("get", Client::get)
        .finish()
    })
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  hebi
    .eval_async(
      r#"
import http

c := http.Client()
print c.get("https://jsonplaceholder.typicode.com/todos/1")
"#,
    )
    .await
    .unwrap();
}
