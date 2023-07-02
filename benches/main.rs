use criterion::criterion_main;

mod benches {
  pub mod fib;
  pub mod primes;
  pub mod startup;
}

#[cfg(enable_slow_bench)]
criterion_main! {
  benches::fib::bench,
  benches::startup::bench,
  benches::primes::bench,
}

#[cfg(not(enable_slow_bench))]
criterion_main! {
  benches::fib::bench,
  benches::startup::bench,
}
