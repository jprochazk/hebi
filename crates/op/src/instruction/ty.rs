#![allow(non_camel_case_types)]

use super::private::Sealed;
use super::*;

/// Determines the width of variable-width operands.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Width {
  Single = 1,
  Double = 2,
  Quad = 4,
}

impl Width {
  pub fn encode(&self, buf: &mut Vec<u8>) {
    match self {
      Width::Single => {}
      Width::Double => buf.push(ops::Wide),
      Width::Quad => buf.push(ops::ExtraWide),
    }
  }

  pub fn encode_into(&self, buf: &mut [u8], offset: usize) -> usize {
    match self {
      Width::Single => 0,
      Width::Double => {
        buf[offset] = ops::Wide;
        1
      }
      Width::Quad => {
        buf[offset] = ops::ExtraWide;
        1
      }
    }
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      Width::Single => "",
      Width::Double => "(w)",
      Width::Quad => "(x)",
    }
  }
}

impl std::ops::BitOr for Width {
  type Output = Self;

  fn bitor(self, rhs: Self) -> Self::Output {
    if self as u8 > rhs as u8 {
      self
    } else {
      rhs
    }
  }
}

pub trait Size: Sealed {
  fn size(width: Width) -> usize;
}

pub trait Operand: Sealed {
  type Decoded: Sized;

  /// Decode the value of this operand from `buf` at the given `offset`,
  /// scaling by `width` as necessary.
  ///
  /// ### Safety
  /// - If `IS_VARIABLE == true`, then caller must ensure that `buf..buf +
  ///   (size_of::<Decoded>() / width)` is a valid range in `buf`.
  /// - If `IS_VARIABLE == false`, then caller must ensure that `buf..buf +
  ///   size_of::<Decoded>()` is a valid range in `buf`.
  unsafe fn decode(buf: &[u8], offset: usize, width: Width) -> Self::Decoded;

  /// Encode the `value` into `buf`.
  fn encode(buf: &mut Vec<u8>, value: Self::Decoded, force_max_width: bool);

  fn encode_into(buf: &mut [u8], offset: usize, value: Self::Decoded) -> usize;

  /// Get the `Width` required to represent this `value`.
  fn width(value: Self::Decoded) -> Width;
}

unsafe fn read_n<const N: usize>(buf: &[u8], start: usize) -> [u8; N] {
  let mut array = [0u8; N];
  array.copy_from_slice(&buf[start..start + N]);
  array
}

macro_rules! read {
  ($buf:ident, $offset:ident, $input:ty, $output:ty) => {
    <$input>::from_le_bytes(read_n($buf, $offset)) as $output
  };
}

macro_rules! write {
  ($buf:ident, $value:ident, $to:ty) => {
    $buf.extend_from_slice(&<$to>::to_le_bytes($value as $to)[..])
  };
}

macro_rules! write_into {
  ($buf:ident, $offset:ident, $value:ident, $to:ty) => {{
    $buf[$offset..$offset + std::mem::size_of::<$to>()]
      .copy_from_slice(&<$to>::to_le_bytes($value as $to)[..]);
    std::mem::size_of::<$to>()
  }};
}

impl Operand for sv {
  type Decoded = i32;

  unsafe fn decode(buf: &[u8], offset: usize, width: Width) -> Self::Decoded {
    match width {
      Width::Single => read!(buf, offset, i8, i32),
      Width::Double => read!(buf, offset, i16, i32),
      Width::Quad => read!(buf, offset, i32, i32),
    }
  }

  fn encode(buf: &mut Vec<u8>, value: Self::Decoded, force_max_width: bool) {
    if force_max_width {
      write!(buf, value, i32);
      return;
    }

    if (i8::MIN as i32) <= value && value <= (i8::MAX as i32) {
      write!(buf, value, i8);
    } else if (i16::MIN as i32) <= value && value <= (i16::MAX as i32) {
      write!(buf, value, i16);
    } else {
      write!(buf, value, i32);
    }
  }

  fn encode_into(buf: &mut [u8], offset: usize, value: Self::Decoded) -> usize {
    if (i8::MIN as i32) <= value && value <= (i8::MAX as i32) {
      write_into!(buf, offset, value, i8)
    } else if (i16::MIN as i32) <= value && value <= (i16::MAX as i32) {
      write_into!(buf, offset, value, i16)
    } else {
      write_into!(buf, offset, value, i32)
    }
  }

  fn width(value: Self::Decoded) -> Width {
    if (i8::MIN as i32) <= value && value <= (i8::MAX as i32) {
      Width::Single
    } else if (i16::MIN as i32) <= value && value <= (i16::MAX as i32) {
      Width::Double
    } else {
      Width::Quad
    }
  }
}

impl Size for uv {
  #[inline]
  fn size(width: Width) -> usize {
    width as usize
  }
}

impl Operand for uv {
  type Decoded = u32;

  unsafe fn decode(buf: &[u8], offset: usize, width: Width) -> Self::Decoded {
    match width {
      Width::Single => read!(buf, offset, u8, u32),
      Width::Double => read!(buf, offset, u16, u32),
      Width::Quad => read!(buf, offset, u32, u32),
    }
  }

  fn encode(buf: &mut Vec<u8>, value: Self::Decoded, force_max_width: bool) {
    if force_max_width {
      write!(buf, value, u32);
      return;
    }

    if (u8::MIN as u32) <= value && value <= (u8::MAX as u32) {
      write!(buf, value, u8);
    } else if (u16::MIN as u32) <= value && value <= (u16::MAX as u32) {
      write!(buf, value, u16);
    } else {
      write!(buf, value, u32);
    }
  }

  fn encode_into(buf: &mut [u8], offset: usize, value: Self::Decoded) -> usize {
    if (u8::MIN as u32) <= value && value < (u8::MAX as u32) {
      write_into!(buf, offset, value, u8)
    } else if (u16::MIN as u32) < value && value < (u16::MAX as u32) {
      write_into!(buf, offset, value, u16)
    } else {
      write_into!(buf, offset, value, u32)
    }
  }

  fn width(value: Self::Decoded) -> Width {
    if (u8::MIN as u32) <= value && value < (u8::MAX as u32) {
      Width::Single
    } else if (u16::MIN as u32) < value && value < (u16::MAX as u32) {
      Width::Double
    } else {
      Width::Quad
    }
  }
}

impl Size for sv {
  #[inline]
  fn size(width: Width) -> usize {
    width as usize
  }
}

macro_rules! fixed {
  ($ty:ty, $decoded:ty) => {
    impl Operand for $ty {
      type Decoded = $decoded;

      unsafe fn decode(buf: &[u8], offset: usize, _: Width) -> Self::Decoded {
        read!(buf, offset, $decoded, $decoded)
      }

      fn encode(buf: &mut Vec<u8>, value: Self::Decoded, _: bool) {
        write!(buf, value, $decoded)
      }

      fn encode_into(buf: &mut [u8], offset: usize, value: Self::Decoded) -> usize {
        write_into!(buf, offset, value, $decoded)
      }

      fn width(_: Self::Decoded) -> Width {
        Width::Single
      }
    }

    impl Size for $ty {
      #[inline]
      fn size(_: Width) -> usize {
        ::std::mem::size_of::<$decoded>()
      }
    }
  };
}

fixed!(sf8, i8);
fixed!(uf8, u8);
fixed!(sf16, i16);
fixed!(uf16, u16);
fixed!(sf32, i32);
fixed!(uf32, u32);

pub struct sv;
pub struct uv;
pub struct sf8;
pub struct uf8;
pub struct sf16;
pub struct uf16;
pub struct sf32;
pub struct uf32;

macro_rules! impl_size_tuple {
  ( () ) => {
    impl Size for () {
      #[inline]
      fn size(_: Width) -> usize {
        0
      }
    }
    impl Sealed for () {}
  };

  (
    ($($v:ident),*)
  ) => {
    impl<$($v),*> Size for ($($v),* ,)
    where
      $(
        $v : Size
      ),*
    {
      #[inline]
      fn size(width: Width) -> usize {
        0 $( + <$v as Size>::size(width) )*
      }
    }
    impl<$($v),*> Sealed for ($($v),* ,)
    where
      $(
        $v : Sealed
      ),*
    {}
  };
}

impl_size_tuple!(());
impl_size_tuple!((A));
impl_size_tuple!((A, B));
impl_size_tuple!((A, B, C));
impl_size_tuple!((A, B, C, D));
impl_size_tuple!((A, B, C, D, E));

// ensure that other crates cannot implement `Operand`
impl Sealed for sv {}
impl Sealed for uv {}
impl Sealed for sf8 {}
impl Sealed for uf8 {}
impl Sealed for sf16 {}
impl Sealed for uf16 {}
impl Sealed for sf32 {}
impl Sealed for uf32 {}
