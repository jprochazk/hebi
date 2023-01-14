#![allow(unused_parens, clippy::unused_unit)]

use ty::*;

use crate::chunk::BytecodeArray;
use crate::disassembly::Disassembly;

pub trait Opcode: private::Sealed {
  const BYTE: u8;

  /// Returns the name of the operand for the purpose of `Display`.
  const NAME: &'static str;

  /// Returns `true` if the given opcode is an instruction prefix.
  const IS_PREFIX: bool;

  /// Returns `true` if the given opcode is a jump instruction.
  const IS_JUMP: bool;

  /// Return the size of operands of `Self`, scaling up variable-width operands
  /// by `width` as needed.
  fn size_of_operands(width: Width) -> usize;

  fn disassemble(operands: &[u8], width: Width) -> Disassembly;
}

pub trait Decode: private::Sealed {
  type Operands: Sized;

  /// Decodes operands from `bytecode` at the given `offset`, scaling up
  /// variable-width operands by `width` as needed.
  fn decode(bytecode: &[u8], offset: usize, width: Width) -> Self::Operands;
}

pub trait Encode: Decode + private::Sealed {
  fn encode(buf: &mut BytecodeArray, operands: Self::Operands);
  fn encode_into(buf: &mut [u8], offset: usize, operands: Self::Operands);
}

// TODO: unify `instruction` and `extra`

macro_rules! instruction {
  (
    $name:ident,
    $name_str:literal,
    $byte:literal,
    ($($operand_name:ident : $operand_type:ty),*),
    is_prefix:$is_prefix:literal,
    is_jump:$is_jump:literal
  ) => {
    pub struct $name;

    impl Opcode for $name {
      const BYTE: u8 = $byte;

      const NAME: &'static str = $name_str;

      const IS_PREFIX: bool = $is_prefix;

      const IS_JUMP: bool = $is_jump;

      #[allow(unused_variables)]
      #[inline]
      fn size_of_operands(width: Width) -> usize {
        0 $( + <$operand_type as Operand>::size(width) )*
      }

      #[allow(unused_mut, unused_variables)]
      fn disassemble(data: &[u8], width: Width) -> Disassembly {
        let mut operands = vec![];
        let mut offset = 0;
        $(
          operands.push(crate::disassembly::Operand {
            name: stringify!($operand_name),
            value: Box::new(unsafe {
              <$operand_type as Operand>::decode(data, offset, width)
            })
          });
          offset += <$operand_type as Operand>::size(width);
        )*

        Disassembly {
          name: $name_str,
          width,
          operands,
          size: 1 + offset,
        }
      }
    }

    impl Decode for $name {
      type Operands = ($(<$operand_type as Operand>::Decoded),*);

      #[allow(unused_mut, unused_assignments)]
      #[inline]
      fn decode(bytecode: &[u8], mut offset: usize, width: Width) -> Self::Operands {
        assert!(bytecode.len() > offset + Self::size_of_operands(width));
        $(
          let $operand_name = unsafe { <$operand_type as Operand>::decode(bytecode, offset, width) };
          offset += <$operand_type as Operand>::size(width);
        )*
        ($($operand_name),*)
      }
    }

    impl Encode for $name {
      #[inline]
      fn encode(buf: &mut Vec<u8>, ($($operand_name),*): Self::Operands) {
        let force_extra_wide = Self::IS_JUMP;

        let prefix = if force_extra_wide {
          Width::Quad
        } else {
          Width::Single $( | <$operand_type as Operand>::width($operand_name) )*
        };

        prefix.encode(buf);
        buf.push(Self::BYTE);
        $(
          <$operand_type as Operand>::encode(buf, $operand_name, force_extra_wide);
        )*
      }

      #[allow(unused_assignments)]
      #[inline]
      fn encode_into(buf: &mut [u8], mut offset: usize, ($($operand_name),*): Self::Operands) {
        let prefix = Width::Single $( | <$operand_type as Operand>::width($operand_name) )* ;
        offset += prefix.encode_into(buf, offset);
        offset += {
          buf[offset] = Self::BYTE;
          1
        };
        $(
          offset += <$operand_type as Operand>::encode_into(buf, offset, $operand_name);
        )*
      }
    }

    impl private::Sealed for $name {}
  };
}

instruction!(Nop, "nop", 0x00, (), is_prefix:false, is_jump:false);
instruction!(Wide, "<illegal (wide)>", 0x01, (), is_prefix:true, is_jump:false);
instruction!(ExtraWide, "<illegal (xwide)>", 0x02, (), is_prefix:true, is_jump:false);
instruction!(LoadConst, "load_const", 0x03, (slot: uv), is_prefix:false, is_jump:false);
instruction!(LoadReg, "load_reg", 0x04, (reg: uv), is_prefix:false, is_jump:false);
instruction!(StoreReg, "store_reg", 0x05, (reg: uv), is_prefix:false, is_jump:false);
instruction!(Jump, "jump", 0x06, (offset: uv), is_prefix:false, is_jump:true);
instruction!(JumpIfFalse, "jump_if_false", 0x07, (offset: uv), is_prefix:false, is_jump:true);
instruction!(Sub, "sub", 0x08, (lhs: uv), is_prefix:false, is_jump:false);
instruction!(Print, "print", 0x09, (args: uv), is_prefix:false, is_jump:false);
instruction!(PushSmallInt, "push_small_int", 0x0A, (value: sf32), is_prefix:false, is_jump:false);
instruction!(CreateEmptyList, "create_empty_list", 0x0B, (), is_prefix: false, is_jump: false);
instruction!(ListPush, "list_push", 0x0C, (list: uv), is_prefix: false, is_jump: false);
instruction!(Ret, "ret", 0xFE, (), is_prefix:false, is_jump:false);
instruction!(Suspend, "suspend", 0xFF, (), is_prefix:false, is_jump:false);

macro_rules! extra {
  ($NAMES:ident, $is_jump:ident, $($inst:ident),* $(,)?) => {
    pub const $NAMES: &[&str] = &[
      $( <$inst>::NAME ),*
    ];

    pub const fn $is_jump(op: u8) -> bool {
      match op {
        $( <$inst>::BYTE => <$inst>::IS_JUMP, )*
        _ => false,
      }
    }

    pub mod ops {
      use super::Opcode;

      $( pub const $inst: u8 = super::$inst::BYTE; )*
    }
  }
}

extra! {
  NAMES, is_jump,
  Nop,
  Wide,
  ExtraWide,
  LoadConst,
  LoadReg,
  StoreReg,
  Jump,
  JumpIfFalse,
  Sub,
  Print,
  PushSmallInt,
  CreateEmptyList,
  ListPush,
  Ret,
  Suspend,
}

pub mod ty {
  #![allow(non_camel_case_types)]

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

  pub trait Operand: super::private::Sealed {
    type Decoded: Sized;
    const IS_VARIABLE: bool;

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
    fn encode(buf: &mut Vec<u8>, value: Self::Decoded, force_extra_wide: bool);

    /// Encode `value` into `buf` at the specified `offset`.
    fn encode_into(buf: &mut [u8], offset: usize, value: Self::Decoded) -> usize;

    /// Get the `Width` required to represent this `value`.
    fn width(value: Self::Decoded) -> Width;

    fn size(width: Width) -> usize;
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
    const IS_VARIABLE: bool = true;

    unsafe fn decode(buf: &[u8], offset: usize, width: Width) -> Self::Decoded {
      match width {
        Width::Single => read!(buf, offset, i8, i32),
        Width::Double => read!(buf, offset, i16, i32),
        Width::Quad => read!(buf, offset, i32, i32),
      }
    }

    fn encode(buf: &mut Vec<u8>, value: Self::Decoded, force_extra_wide: bool) {
      if force_extra_wide {
        write!(buf, value, i32);
        return;
      }

      if (i8::MIN as i32) <= value && value <= (i8::MAX as i32) {
        write!(buf, value, i8);
      } else if (i16::MIN as i32) <= value && value <= i16::MAX as i32 {
        write!(buf, value, i16);
      } else {
        write!(buf, value, i32);
      }
    }

    fn encode_into(buf: &mut [u8], offset: usize, value: Self::Decoded) -> usize {
      if (i8::MIN as i32) <= value && value <= (i8::MAX as i32) {
        write_into!(buf, offset, value, i8)
      } else if (i16::MIN as i32) <= value && value <= i16::MAX as i32 {
        write_into!(buf, offset, value, i16)
      } else {
        write_into!(buf, offset, value, i32)
      }
    }

    fn width(value: Self::Decoded) -> Width {
      if (i8::MIN as i32) <= value && value <= (i8::MAX as i32) {
        Width::Single
      } else if (i16::MIN as i32) <= value && value <= i16::MAX as i32 {
        Width::Double
      } else {
        Width::Quad
      }
    }

    fn size(width: Width) -> usize {
      width as usize
    }
  }

  impl Operand for uv {
    type Decoded = u32;
    const IS_VARIABLE: bool = true;

    unsafe fn decode(buf: &[u8], offset: usize, width: Width) -> Self::Decoded {
      match width {
        Width::Single => read!(buf, offset, u8, u32),
        Width::Double => read!(buf, offset, u16, u32),
        Width::Quad => read!(buf, offset, u32, u32),
      }
    }

    fn encode(buf: &mut Vec<u8>, value: Self::Decoded, force_extra_wide: bool) {
      if force_extra_wide {
        write!(buf, value, i32);
        return;
      }

      if (u8::MIN as u32) <= value && value < (u8::MAX as u32) {
        write!(buf, value, u8);
      } else if (u16::MIN as u32) < value && value < (u16::MAX as u32) {
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

    fn size(width: Width) -> usize {
      width as usize
    }
  }

  macro_rules! fixed {
    ($ty:ty, $decoded:ty) => {
      impl Operand for $ty {
        type Decoded = $decoded;
        const IS_VARIABLE: bool = false;

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

  // ensure that other crates cannot implement `Operand`
  impl super::private::Sealed for sv {}
  impl super::private::Sealed for uv {}
  impl super::private::Sealed for sf8 {}
  impl super::private::Sealed for uf8 {}
  impl super::private::Sealed for sf16 {}
  impl super::private::Sealed for uf16 {}
  impl super::private::Sealed for sf32 {}
  impl super::private::Sealed for uf32 {}
}

mod private {
  pub trait Sealed {}
}

pub mod disassembly {}
