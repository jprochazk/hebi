pub fn num_digits(v: usize) -> usize {
  use core::iter::successors;

  successors(Some(v), |&n| (n >= 10).then_some(n / 10)).count()
}
