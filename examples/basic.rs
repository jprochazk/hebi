use hebi::prelude::*;

fn main() {
  let mut hebi = Hebi::new();
  hebi
    .eval(
      r#"
print "Hello, world!"
"#,
    )
    .unwrap();
}
