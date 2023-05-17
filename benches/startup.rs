use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hebi::module::NativeModule;
use hebi::{Hebi, IntoValue, Result, Scope, Value};

fn example<'cx>(scope: &'cx Scope<'cx>) -> Result<Value<'cx>> {
  let value = 100i32;
  value.into_value(scope.cx())
}

fn add1<'cx>(scope: &'cx Scope<'cx>) -> Result<Value<'cx>> {
  let value = scope
    .argument(0)
    .ok_or_else(|| hebi::error!("Missing argument 0"))?;
  let value = value
    .as_int()
    .ok_or_else(|| hebi::error!("First argument must be an integer"))?;

  let value = value + 1;

  value.into_value(scope.cx())
}

pub fn benchmark(c: &mut Criterion) {
  c.bench_function("startup empty", |b| {
    b.iter(|| {
      black_box(Hebi::new());
    })
  });

  c.bench_function("startup 1 module", |b| {
    b.iter(|| {
      let mut hebi = Hebi::new();

      let module = NativeModule::builder("test")
        .function("example", example)
        .function("add1", add1)
        .finish();

      hebi.register(&module);

      black_box(hebi);
    })
  });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
