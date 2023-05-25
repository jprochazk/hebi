use criterion::{criterion_group, Criterion};
use hebi::Hebi;

pub fn primes(c: &mut Criterion) {
  c.bench_function("primes", |b| {
    let mut hebi = Hebi::new();

    let chunk = hebi
      .compile(indoc::indoc! {
        r#"
          fn primes(max_number):
            prime_mask := []
            prime_mask.extend(max_number+1, true)

            prime_mask[0] = false
            prime_mask[1] = false

            total_primes_found := 0
            for p in 2..=max_number:
              if !prime_mask[p]: continue

              total_primes_found += 1

              i := 2 * p
              while i < max_number + 1:
                prime_mask[i] = false
                i += p

            return total_primes_found

          primes(1000000)
        "#,
      })
      .unwrap();

    b.iter(|| {
      let answer = hebi.run(chunk.clone()).unwrap().as_int().unwrap();
      assert_eq!(answer, 78_498);
    })
  });
}

criterion_group!(bench, primes);
