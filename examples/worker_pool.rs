use futures_util::future::join_all;
use hebi::prelude::*;

#[tokio::main]
async fn main() {
  let client = reqwest::Client::new();

  let module = NativeModule::builder("http")
    .async_function("get", move |scope| get(scope, client.clone()))
    .finish();

  const CONCURRENCY: usize = 16;

  let pool = WorkerPool::new(&[module], CONCURRENCY);

  let mut handles = vec![];
  for index in 0..CONCURRENCY {
    let pool = pool.clone();
    let source = format!(
      r#"
import http
http.get("https://jsonplaceholder.typicode.com/todos/{index}")
"#
    );

    let task = tokio::spawn(async move {
      let mut hebi = pool.get().await;
      let response = hebi.eval_async(&source).await.unwrap();
      println!("{response}");
      pool.put(hebi);
    });

    handles.push(task);
  }

  join_all(handles).await;
}

#[derive(Clone)]
struct WorkerPool {
  tx: flume::Sender<Hebi>,
  rx: flume::Receiver<Hebi>,
}

impl WorkerPool {
  pub fn new(modules: &[NativeModule], capacity: usize) -> Self {
    let (tx, rx) = flume::bounded(capacity);
    for _ in 0..capacity {
      let mut worker = Hebi::new();
      for module in modules {
        worker.register(module);
      }
      tx.send(worker).unwrap();
    }

    Self { tx, rx }
  }

  pub async fn get(&self) -> Hebi {
    self.rx.recv_async().await.unwrap()
  }

  pub fn put(&self, worker: Hebi) {
    self.tx.send(worker).unwrap()
  }
}

async fn get(scope: Scope<'_>, client: reqwest::Client) -> hebi::Result<Str<'_>> {
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
