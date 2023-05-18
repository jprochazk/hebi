use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hebi::module::NativeModule;
use hebi::{Hebi, Result, Scope};

fn example(_: Scope) -> i32 {
  100i32
}

fn add1(scope: Scope) -> Result<i32> {
  let value = scope.param::<i32>(0)?;
  Ok(value + 1)
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
