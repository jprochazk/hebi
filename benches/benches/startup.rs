use criterion::{black_box, criterion_group, Criterion};
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
  let modules = (0..100)
    .map(|i| {
      NativeModule::builder(format!("module_{i}"))
        .function("example", example)
        .function("add1", add1)
        .finish()
    })
    .collect::<Vec<_>>();

  c.bench_function("startup empty", |b| {
    b.iter(|| {
      black_box(Hebi::new());
    })
  });

  c.bench_function("startup + register 1 module", |b| {
    b.iter(|| {
      let mut hebi = Hebi::new();

      hebi.register(&modules[0]);

      black_box(hebi);
    })
  });

  c.bench_function("startup + register 10 modules", |b| {
    b.iter(|| {
      let mut hebi = Hebi::new();

      for module in modules[0..10].iter() {
        hebi.register(module);
      }

      black_box(hebi);
    })
  });

  c.bench_function("startup + register 100 modules", |b| {
    b.iter(|| {
      let mut hebi = Hebi::new();

      for module in modules.iter() {
        hebi.register(module);
      }

      black_box(hebi);
    })
  });
}

criterion_group!(bench, benchmark);
