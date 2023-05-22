use hebi::{Hebi, NativeModule, Scope, Value};

fn main() {
  fn example(scope: Scope) {
    if scope.num_args() > 0 {
      if let Some(value) = scope.param::<Option<Value>>(0).unwrap() {
        println!("Got: {value}");
        return;
      }
    }
    println!("Got nothing");
  }

  let module = NativeModule::builder("test")
    .function("example", example)
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  hebi
    .eval(
      r#"
from test import example

example()
example(none)
example("some")
"#,
    )
    .unwrap();
}
