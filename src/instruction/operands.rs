use std::ops::BitOr;

use super::opcodes::{self, Instruction};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Width {
  Normal,
  Wide16,
  Wide32,
}

impl Width {
  #[inline]
  pub fn size(&self) -> usize {
    match self {
      Width::Normal => 1,
      Width::Wide16 => 2,
      Width::Wide32 => 4,
    }
  }

  #[inline]
  pub fn encode(&self, buf: &mut Vec<u8>) {
    match self {
      Width::Normal => {}
      Width::Wide16 => buf.push(opcodes::Wide16::BYTE),
      Width::Wide32 => buf.push(opcodes::Wide32::BYTE),
    }
  }
}

impl BitOr for Width {
  type Output = Width;

  #[inline]
  fn bitor(self, rhs: Self) -> Self::Output {
    if self >= rhs {
      self
    } else {
      rhs
    }
  }
}

pub trait Operand {
  type Decoded: Sized;

  fn encode(&self, buf: &mut Vec<u8>, width: Width);
  fn decode(buf: &[u8], width: Width) -> Self::Decoded;
  fn width(&self) -> Width;
}

#[inline]
fn read_n<const N: usize>(buf: &[u8]) -> [u8; N] {
  let mut array = [0u8; N];
  array.copy_from_slice(buf);
  array
}

macro_rules! decode {
  ($buf:ident, $input:ty) => {
    <$input>::from_le_bytes(read_n($buf)) as _
  };
}

macro_rules! encode {
  ($buf:ident, $value:expr, $to:ty) => {
    $buf.extend_from_slice(&<$to>::to_le_bytes($value as $to)[..])
  };
}

impl Operand for i32 {
  type Decoded = i32;

  fn encode(&self, buf: &mut Vec<u8>, width: Width) {
    match width {
      Width::Normal => encode!(buf, *self, i8),
      Width::Wide16 => encode!(buf, *self, i16),
      Width::Wide32 => encode!(buf, *self, i32),
    }
  }

  fn decode(buf: &[u8], width: Width) -> Self::Decoded {
    match width {
      Width::Normal => decode!(buf, i8),
      Width::Wide16 => decode!(buf, i16),
      Width::Wide32 => decode!(buf, i32),
    }
  }

  fn width(&self) -> Width {
    if (i8::MIN as Self::Decoded) <= *self && *self <= (i8::MAX as Self::Decoded) {
      Width::Normal
    } else if (i16::MIN as Self::Decoded) <= *self && *self <= (i16::MAX as Self::Decoded) {
      Width::Wide16
    } else {
      Width::Wide32
    }
  }
}

impl Operand for u32 {
  type Decoded = u32;

  #[inline]
  fn encode(&self, buf: &mut Vec<u8>, width: Width) {
    match width {
      Width::Normal => encode!(buf, *self, u8),
      Width::Wide16 => encode!(buf, *self, u16),
      Width::Wide32 => encode!(buf, *self, u32),
    }
  }

  #[inline]
  fn decode(buf: &[u8], width: Width) -> Self::Decoded {
    match width {
      Width::Normal => decode!(buf, u8),
      Width::Wide16 => decode!(buf, u16),
      Width::Wide32 => decode!(buf, u32),
    }
  }

  #[inline]
  fn width(&self) -> Width {
    if *self <= (u8::MAX as Self::Decoded) {
      Width::Normal
    } else if *self <= (u16::MAX as Self::Decoded) {
      Width::Wide16
    } else {
      Width::Wide32
    }
  }
}

impl Operand for () {
  type Decoded = ();

  #[inline]
  fn encode(&self, buf: &mut Vec<u8>, width: Width) {
    let _ = (buf, width);
  }

  #[inline]
  fn decode(buf: &[u8], width: Width) -> Self::Decoded {
    let _ = (buf, width);
  }

  #[inline]
  fn width(&self) -> Width {
    Width::Normal
  }
}

macro_rules! impl_for_tuple {
  ($($ty:ident),+) => {
    impl<$($ty,)+> Operand for ($($ty,)+)
    where
      $($ty : Operand,)+
    {
      type Decoded = ($(<$ty as Operand>::Decoded,)+);

      #[inline]
      #[allow(non_snake_case)]
      fn encode(&self, buf: &mut Vec<u8>, width: Width) {
        let ($($ty,)+) = self;
        $(
          ($ty).encode(buf, width);
        )+
      }

      #[inline]
      #[allow(non_snake_case)]
      fn decode(buf: &[u8], width: Width) -> Self::Decoded {
        let mut offset = 0;
        $(
          let $ty = <$ty as Operand>::decode(&buf[offset..], width);
          offset += width.size();
        )+
        let _ = offset;
        ($($ty,)+)
      }

      #[inline]
      #[allow(non_snake_case)]
      fn width(&self) -> Width {
        let ($($ty,)+) = self;

        let mut width = Width::Normal;
        $(
          width = width | <$ty as Operand>::width($ty);
        )+
        width
      }
    }
  }
}

impl_for_tuple!(A);
impl_for_tuple!(A, B);
impl_for_tuple!(A, B, C);
impl_for_tuple!(A, B, C, D);
