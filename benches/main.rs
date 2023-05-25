use criterion::criterion_main;

mod benches {
  pub mod fib;
  pub mod primes;
  pub mod startup;
}

criterion_main! {
  benches::fib::bench,
  benches::startup::bench,
  benches::primes::bench,
}
