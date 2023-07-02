use hebi::prelude::*;

fn main() {
  async fn map(mut scope: Scope<'_>) -> hebi::Result<List<'_>> {
    let (list, cb) = scope.params::<(List, Any)>()?;

    let out = scope.new_list(list.len());
    for i in 0..list.len() {
      let value = scope.call(cb.clone(), &[list.get(i).unwrap()]).await?;
      out.push(value);
    }

    Ok(out)
  }

  let module = NativeModule::builder("test")
    .async_function("map", map)
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  let result = hebi
    .eval(
      r#"
from test import map

fn add1(v):
  return v + 1

map([0, 1, 2], add1)
"#,
    )
    .unwrap();

  println!("{result:?}")
}
