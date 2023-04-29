use std::fmt::Display;

#[cfg(test)]
macro_rules! assert_snapshot {
  ($body:expr) => {
    if cfg!(feature = "__assert_snapshots") {
      insta::assert_snapshot!($body);
    } else {
      let _ = $body;
    }
  };
}

#[cfg(test)]
macro_rules! assert_debug_snapshot {
  ($body:expr) => {
    if cfg!(feature = "__assert_snapshots") {
      insta::assert_debug_snapshot!($body);
    } else {
      let _ = $body;
    }
  };
}

/* macro_rules! static_assert_size {
  ($T:ty, $S:ty) => {
    const _: fn() = || {
      let _ = ::core::mem::transmute::<$T, $S>;
    };
  };
} */

pub struct Join<Iter, Sep>(pub Iter, pub Sep);

impl<Iter, Sep> Display for Join<Iter, Sep>
where
  Iter: Iterator + Clone,
  <Iter as Iterator>::Item: Display,
  Sep: Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let sep = &self.1;
    let mut peekable = self.0.clone().peekable();
    while let Some(item) = peekable.next() {
      write!(f, "{item}")?;
      if peekable.peek().is_some() {
        write!(f, "{sep}")?;
      }
    }
    Ok(())
  }
}

pub trait JoinIter: Sized {
  fn join<Sep>(&self, sep: Sep) -> Join<Self, Sep>;
}

impl<Iter> JoinIter for Iter
where
  Iter: Sized + Iterator + Clone,
{
  fn join<Sep>(&self, sep: Sep) -> Join<Self, Sep> {
    Join(self.clone(), sep)
  }
}

pub fn num_digits(v: usize) -> usize {
  use std::iter::successors;

  successors(Some(v), |&n| (n >= 10).then_some(n / 10)).count()
}
