use core::fmt::{Debug, Display};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct u24([u8; 3]);

impl u24 {
  pub const MAX: u24 = u24([255, 255, 255]);
}

impl From<u8> for u24 {
  fn from(value: u8) -> Self {
    Self([value, 0, 0])
  }
}

impl From<u16> for u24 {
  fn from(value: u16) -> Self {
    let mut bytes = [0; 3];
    bytes.copy_from_slice(&value.to_le_bytes()[0..=1]);
    u24(bytes)
  }
}

impl TryFrom<u32> for u24 {
  type Error = ();

  #[inline]
  fn try_from(value: u32) -> Result<Self, Self::Error> {
    u24::try_from(value as usize)
  }
}

impl TryFrom<usize> for u24 {
  type Error = ();

  #[inline]
  fn try_from(value: usize) -> Result<Self, Self::Error> {
    if value < (1 << 24) {
      let mut bytes = [0; 3];
      bytes.copy_from_slice(&value.to_le_bytes()[0..=2]);
      Ok(u24(bytes))
    } else {
      Err(())
    }
  }
}

impl From<u24> for u32 {
  #[inline]
  fn from(value: u24) -> Self {
    let mut bytes = [0; 4];
    bytes[0..=2].copy_from_slice(&value.0);
    u32::from_le_bytes(bytes)
  }
}

impl From<u24> for usize {
  fn from(value: u24) -> Self {
    u32::from(value) as usize
  }
}

impl From<u24> for u64 {
  fn from(value: u24) -> Self {
    u32::from(value) as u64
  }
}

impl Display for u24 {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Display::fmt(&u32::from(*self), f)
  }
}

impl Debug for u24 {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Debug::fmt(&u32::from(*self), f)
  }
}

#[cfg(test)]
mod tests {
  #![allow(non_snake_case)]

  use super::*;

  #[test]
  #[should_panic]
  fn test_try_from_u32__too_large() {
    let v = u32::MAX;
    u24::try_from(v).unwrap();
  }

  #[test]
  fn test_try_from_u32__in_range() {
    let v = u16::MAX as u32;
    u24::try_from(v).unwrap();
  }

  #[test]
  fn test_try_from_u32__round_trip() {
    let v = u16::MAX as u32;
    let v = u24::try_from(v).unwrap();
    let v = u32::from(v);
    assert_eq!(v, u16::MAX as u32);

    let v = u16::MIN as u32;
    let v = u24::try_from(v).unwrap();
    let v = u32::from(v);
    assert_eq!(v, u16::MIN as u32);
  }
}
