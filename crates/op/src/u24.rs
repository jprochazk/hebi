use std::fmt::{Debug, Display};
use std::ops::AddAssign;

/// A 24-bit unsigned integer type.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct u24([u8; 3]);

impl u24 {
  pub const MAX: u24 = u24([0xFF, 0xFF, 0xFF]);
  pub const MIN: u24 = u24([0x00, 0x00, 0x00]);

  /// Create a new `u24`
  ///
  /// ### Panics
  ///
  /// Panics if `v` is larger than `u24::MAX`.
  /// Use `try_from` if you wish to handle this case.
  pub fn new(v: u32) -> Self {
    Self::try_from(v).unwrap()
  }
}

impl From<u8> for u24 {
  fn from(value: u8) -> Self {
    Self::from(value as u16)
  }
}

impl From<u16> for u24 {
  fn from(value: u16) -> Self {
    // SAFETY: This is safe because `value` is always less than u24::MAX,
    // so an `unwrap` would never panic.
    unsafe { Self::try_from(value as u32).unwrap_unchecked() }
  }
}

impl From<u24> for u32 {
  fn from(v: u24) -> Self {
    let [a, b, c] = v.0;
    ((a as u32) << 16) + ((b as u32) << 8) + (c as u32)
  }
}

impl TryFrom<u32> for u24 {
  type Error = ();

  fn try_from(value: u32) -> Result<Self, Self::Error> {
    if value > 0x00FFFFFF {
      return Err(());
    }

    Ok(Self([
      (value & 0x00ff0000 >> 16) as u8,
      (value & 0x0000ff00 >> 8) as u8,
      (value & 0x000000ff) as u8,
    ]))
  }
}

impl AddAssign<u32> for u24 {
  fn add_assign(&mut self, rhs: u32) {
    let mut temp = u32::from(*self);
    temp.add_assign(rhs);
    *self = Self::try_from(temp).unwrap();
  }
}

impl Debug for u24 {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <u32 as Debug>::fmt(&u32::from(*self), f)
  }
}

impl Display for u24 {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <u32 as Display>::fmt(&u32::from(*self), f)
  }
}

static_assertions::assert_eq_size!(u24, [u8; 3]);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn u24_ord() {
    let a = u24::new(16_000_000);
    let b = u24::new(16_000);
    let c = u24::new(16);
    assert!(a > b);
    assert!(b > c);
    assert!(a > c);
  }

  #[test]
  fn u24_from_u32() {
    let as_u32 = 0x00FFFFFF_u32;
    let as_u24 = u24::try_from(as_u32).unwrap();
    assert_eq!(as_u24.0, [0xFF, 0xFF, 0xFF]);
    assert_eq!(as_u32, as_u24.into());
  }

  #[test]
  #[should_panic]
  fn u24_from_u32_overflow() {
    let as_u32 = 0x00FFFFFF_u32 + 1;
    let _ = u24::try_from(as_u32).expect("u24 overflow");
  }
}
