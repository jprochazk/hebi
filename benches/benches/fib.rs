use criterion::{black_box, criterion_group, Criterion};
use hebi::prelude::*;

pub fn fib_15(c: &mut Criterion) {
  c.bench_function("fib(15)", |b| {
    let mut hebi = Hebi::new();

    let chunk = hebi
      .compile(indoc::indoc! {
        r#"#!hebi
          fn fib(n):
            if n <= 1:
              return n
            else:
              return fib(n - 2) + fib(n - 1)
          
          fib(15)
        "#,
      })
      .unwrap();

    b.iter(|| {
      black_box(hebi.run(chunk.clone()).unwrap());
    })
  });
}

pub fn fib_20(c: &mut Criterion) {
  c.bench_function("fib(20)", |b| {
    let mut hebi = Hebi::new();

    let chunk = hebi
      .compile(indoc::indoc! {
        r#"
          fn fib(n):
            if n <= 1:
              return n
            else:
              return fib(n - 2) + fib(n - 1)
          
          fib(20)
        "#,
      })
      .unwrap();

    b.iter(|| {
      black_box(hebi.run(chunk.clone()).unwrap());
    })
  });
}

pub fn fib_iter_big(c: &mut Criterion) {
  c.bench_function("fib_iter(10_000_000)", |b| {
    let mut hebi = Hebi::new();

    let chunk = hebi
      .compile(indoc::indoc! {
        r#"
          fn fib(n):
            a := 0.0
            b := 1.0
            i := 0
            while i < n:
              c := a + b
              a = b
              b = c
              i = i + 1
            return a
          fib(10000000)
        "#
      })
      .unwrap();

    b.iter(|| {
      black_box(hebi.run(chunk.clone()).unwrap());
    })
  });
}

criterion_group!(bench, fib_15, fib_20, fib_iter_big);
