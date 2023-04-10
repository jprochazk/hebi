macro_rules! static_assert_size {
  ($T:ty, $S:ty) => {
    const _: fn() = || {
      let _ = ::core::mem::transmute::<$T, $S>;
    };
  };
}
