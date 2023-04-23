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

macro_rules! static_assert_size {
  ($T:ty, $S:ty) => {
    const _: fn() = || {
      let _ = ::core::mem::transmute::<$T, $S>;
    };
  };
}
