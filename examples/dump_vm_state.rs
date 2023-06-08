use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use hebi::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
  let mut hebi = Hebi::new();

  let code = r#"
args := ctx.args()

if args:
  ws_sender.send("!pong " + args.join(" "))
else:
  ws_sender.send("!pong")
"#;

  eprintln!("{hebi:#?}");

  match AssertUnwindSafe(hebi.eval_async(code)).catch_unwind().await {
    Ok(result) => match result {
      Ok(value) => println!("Value: {value}"),
      Err(e) => println!("Error: {e}"),
    },
    Err(e) => {
      eprintln!("{hebi:?}");
      panic!("{e:?}");
    }
  }

  eprintln!("{hebi:#?}");

  Ok(())
}
