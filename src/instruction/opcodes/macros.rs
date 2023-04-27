macro_rules! __struct {
  ($name:ident) => {
    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    pub struct $name;
  };
  ($name:ident ($($operand:ident : $ty:ty),+)) => {
    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    pub struct $name {
      $(pub $operand : $ty),+
    }
  };
}

/* macro_rules! __count {
  () => (0);
  ($head:ident $($tail:ident)*) => ((1 + __count!($($tail)*)));
} */

macro_rules! __last {
  ($tail:ident) => ($tail);
  ($head:ident $($tail:ident)+) => (__last!($($tail)+));
}

macro_rules! instructions {
  ($symbolic:ident, $Opcode:ident; $($name:ident $(($($operand:ident : $ty:ty),+))?),* $(,)?) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(u8)]
    pub enum $Opcode {
      $($name),*
    }

    impl $Opcode {
      pub fn new(v: u8) -> $Opcode {
        match Self::try_from(v) {
          Ok(v) => v,
          Err(()) => panic!("illegal instruction 0x{v:X}"),
        }
      }
    }

    impl TryFrom<u8> for $Opcode {
      type Error = ();
      fn try_from(value: u8) -> Result<$Opcode, Self::Error> {
        use $Opcode::*;
        if value > (__last!($($name)*) as u8) {
          return Err(());
        }
        Ok(unsafe { std::mem::transmute::<u8, $Opcode>(value) })
      }
    }

    const _: () = {
      let _ = ::core::mem::transmute::<$Opcode, u8>;
    };

    pub mod $symbolic {
      $(
        pub use super::$name;
      )*
    }

    $(
      __struct!($name $(($($operand : $ty),+))?);
      impl private::Sealed for $name {}
      impl Operands for $name {
        type Operands = ($($($ty,)+)?);
      }
      impl Encode for $name {
        #[inline]
        fn encode(&self, buf: &mut Vec<u8>) {
          let Self { $($($operand,)+)? } = *self;
          let operands = ($($($operand,)+)?);
          let width = operands.width();
          width.encode(buf);
          buf.push(Self::OPCODE as u8);
          operands.encode(buf, width);
        }
      }
      impl Decode for $name {
        #[inline]
        fn decode(buf: &[u8], width: Width) -> <Self::Operands as Operand>::Decoded {
          Self::Operands::decode(buf, width)
        }
      }
      impl Instruction for $name {
        const OPCODE: $Opcode = $Opcode::$name;
        const NAME: &'static str = paste!(stringify!([<$name:snake>]));
      }
    )*
  };
}

macro_rules! operand_type {
  ($name:ident, $inner:ty) => {
    #[derive(Debug, Clone, Copy)]
    pub struct $name(pub $inner);

    impl Operand for $name {
      type Decoded = $inner;

      #[inline]
      fn encode(&self, buf: &mut Vec<u8>, width: Width) {
        self.0.encode(buf, width)
      }

      #[inline]
      fn encode_into(&self, buf: &mut [u8], width: Width) {
        self.0.encode_into(buf, width)
      }

      #[inline]
      fn decode(buf: &[u8], width: Width) -> Self::Decoded {
        <$inner as Operand>::decode(buf, width)
      }

      #[inline]
      fn width(&self) -> Width {
        <$inner as Operand>::width(&self.0)
      }
    }
  };
}
