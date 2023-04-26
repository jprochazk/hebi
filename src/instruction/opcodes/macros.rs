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
      $($operand : $ty),+
    }
  };
}

macro_rules! instructions {
  ($($name:ident $(($($operand:ident : $ty:ty),+))?),* $(,)?) => {
    #[repr(u8)]
    enum Opcodes {
      $($name),*
    }

    const _: () = {
      let _ = ::core::mem::transmute::<Opcodes, u8>;
    };

    $(
      __struct!($name $(($($operand : $ty),+))?);
      impl Operands for $name {
        type Operands = ($($($ty,)+)?);
      }
      impl Encode for $name {
        fn encode(&self, buf: &mut Vec<u8>) {
          let Self { $($($operand,)+)? } = *self;
          let operands = ($($($operand,)+)?);
          let width = operands.width();
          width.encode(buf);
          buf.push(Self::BYTE);
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
        const BYTE: u8 = Opcodes::$name as u8;
        const NAME: &'static str = paste!(stringify!([<$name:snake>]));
      }
    )*
  };
}

macro_rules! operand_type {
  ($name:ident, $inner:ty) => {
    #[derive(Clone, Copy)]
    pub struct $name(pub $inner);

    impl Operand for $name {
      type Decoded = $inner;

      fn encode(&self, buf: &mut Vec<u8>, width: Width) {
        self.0.encode(buf, width)
      }

      fn decode(buf: &[u8], width: Width) -> Self::Decoded {
        <$inner as Operand>::decode(buf, width)
      }

      fn width(&self) -> Width {
        <$inner as Operand>::width(&self.0)
      }
    }
  };
}

macro_rules! jump_instructions {
  ($($ty:ident),* $(,)?) => {
    $(
      impl $ty {
        pub fn empty() -> Self {
          Self { offset: Offset(0) }
        }
      }

      impl JumpInstruction for $ty {
        type Const = paste!([<$ty Const>]);

        fn update_offset(&mut self, offset: Offset) {
          self.offset = offset;
        }

        fn to_const(self) -> Self::Const {
          unsafe { std::mem::transmute::<Self, Self::Const>(self) }
        }
      }

      impl JumpInstruction for paste!([<$ty Const>]) {
        type Const = Self;

        fn update_offset(&mut self, _: Offset) {
          panic!("cannot update offset of {}", paste!(stringify!([<$ty Const>])));
        }

        fn to_const(self) -> Self::Const { self }
      }
    )*
  };
}
