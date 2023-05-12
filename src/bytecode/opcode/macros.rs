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

macro_rules! __count {
  () => (0);
  ($head:ident $($tail:ident)*) => ((1 + __count!($($tail)*)));
}

macro_rules! __last {
  ($tail:ident) => ($tail);
  ($head:ident $($tail:ident)+) => (__last!($($tail)+));
}

macro_rules! __get_constant {
  ($constants:ident; ($operand:ident, Constant) $(($tail:ident, $tail_ty:ident))*) => (
    Some($constants[$operand.0 as usize].clone())
  );
  ($constants:ident; ($head:ident, $head_ty:ident) $(($tail:ident, $tail_ty:ident))*) => (
    __get_constant!($constants; $(($tail, $tail_ty))*)
  );
  ($constants:ident; ) => (
    None
  );
}

macro_rules! __patch_register {
  ($width:ident, $map:ident, $buf:expr, Register) => {
    let value = Register::decode(&*$buf, $width);
    let value = $map[value.0 as usize] as u32;
    value.encode_into($buf, $width);
  };
  ($width:ident, $map:ident, $buf:expr, $ty:ident) => {};
}

macro_rules! instructions {
  ($patch_registers:ident, $symbolic:ident, $decode:ident, $Opcode:ident; $($name:ident $(($($operand:ident : $ty:ident),+))?),* $(,)?) => {
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
      use super::*;

      $(
        __struct!($name $(($($operand : $ty),+))?);
        impl private::Sealed for $name {}
        impl Operands for $name {
          type Operands = ($($($ty,)+)?);
        }
        impl Instruction for $name {
          #[inline]
          fn opcode(&self) -> $Opcode {
            $Opcode::$name
          }
          #[inline]
          fn encode(&self, buf: &mut Vec<u8>) {
            let Self { $($($operand,)+)? } = *self;
            let operands = ($($($operand,)+)?);
            let width = operands.width();
            width.encode(buf);
            buf.push($Opcode::$name as u8);
            operands.encode(buf, width);
          }
        }
        impl disasm::Disassemble for $name {
          #[allow(unused_variables)]
          fn disassemble(&self, constants: &[crate::value::constant::Constant]) -> disasm::Instruction {
            let Self { $($($operand,)+)? } = self;

            let _name: &'static str = ::paste::paste!(stringify!([<$name:snake>]));
            let _operands: Vec<&dyn ::std::fmt::Display> = vec![$($(&*$operand),+)?];
            let _constant: Option<crate::value::constant::Constant> = __get_constant!(constants; $($(($operand, $ty))+)?);
            let _width = ($($($operand.clone(),)+)?).width();

            disasm::Instruction {
              name: _name,
              operands: _operands,
              constant: _constant,
              width: _width,
            }
          }
        }
      )*

      #[allow(unused_parens)]
      pub fn decode(buf: &[u8]) -> Option<(Box<dyn Instruction>, &[u8])> {
        assert!(!buf.is_empty());

        let (width, opcode, operands) = read_instruction(buf)?;

        match opcode {
          $(
            $Opcode::$name => {
              let ($($($operand,)+)?) = <<$name as Operands>::Operands>::decode(operands, width);
              let instruction = Box::new($name { $($($operand: <$ty>::new($operand.0),)+)? });
              let remainder = &operands[__count!($($($operand)+)?) * width.size()..];
              return Some((instruction, remainder));
            }
          )*
        }
      }
    }

    pub fn $patch_registers(buf: &mut [u8], map: &[usize]) {
      let mut remaining = buf;
      while !remaining.is_empty() {
        let (width, opcode, operands) = read_instruction_mut(remaining).unwrap();
        let mut operand_index = 0;
        match opcode {
          $(
            $Opcode::$name => {
              $($(
                __patch_register!(
                  width,
                  map,
                  &mut operands[operand_index*width.size()..],
                  $ty
                );
                operand_index += 1;
              )+)?
              remaining = &mut operands[operand_index*width.size()..];
            }
          )*
        }
      }
    }
  };
}

macro_rules! operand_type {
  ($name:ident, $inner:ty, $fmt:literal) => {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct $name(pub $inner);

    impl $name {
      pub fn new(value: $inner) -> Self {
        Self(value)
      }
    }

    impl Operand for $name {
      #[inline]
      fn encode(&self, buf: &mut Vec<u8>, width: Width) {
        self.0.encode(buf, width)
      }

      #[inline]
      fn encode_into(&self, buf: &mut [u8], width: Width) {
        self.0.encode_into(buf, width)
      }

      #[inline]
      fn decode(buf: &[u8], width: Width) -> Self {
        Self(<$inner as Operand>::decode(buf, width))
      }

      #[inline]
      fn width(&self) -> Width {
        <$inner as Operand>::width(&self.0)
      }
    }

    impl ::std::fmt::Display for $name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, $fmt, v = self.0)
      }
    }
  };
}
