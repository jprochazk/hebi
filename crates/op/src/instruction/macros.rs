macro_rules! to_snake_case_str {
  ($name:ident) => {
    paste! { stringify!([<$name:snake>]) }
  };
}

macro_rules! instruction_struct {
  (
    $name:ident ()
  ) => {
    #[derive(Debug, Clone)]
    pub struct $name;
  };
  (
    $name:ident ($( $operand:ident : $ty:ty ),*)
  ) => {
    #[derive(Debug, Clone)]
    pub struct $name {
      $( pub $operand : <$ty as Operand>::Decoded ),*
    }
  };
}

macro_rules! impl_encode_into_for_jump {
  ( :jump $name:ident ($( $operand:ident : $ty:ty ),*) ) => {
    impl EncodeInto for $name {
      #[inline]
      fn encode_into(buf: &mut [u8], ($($operand),*): <$name as Decode>::Decoded) {
        let prefix = Width::Single $( | <$ty as Operand>::width($operand) )* ;
        let mut pc = prefix.encode_into(buf, 0);
        pc += {
          buf[pc] = ops::$name;
          1
        };
        $(
          pc += <$ty as Operand>::encode_into(buf, pc, $operand);
        )*
        drop(pc);
      }
    }
  };
  ( $name:ident ($( $operand:ident : $ty:ty ),*) ) => {};
}

macro_rules! instruction_base {
  (
    $Instruction:ident, $Handler:ident;
    $(:$jump:ident)? $name:ident ($( $operand:ident : $ty:ty )*) $(= $index:literal)?
  ) => {
    instruction_struct!($name ($($operand : $ty),*));

    impl Disassemble for $name {
      #[allow(unused_mut, unused_variables)]
      fn disassemble(buf: &[u8], pc: usize, width: Width) -> Disassembly {
        let start_pc = pc;
        let mut pc = pc + 1;
        let mut operands = vec![];
        $(
          operands.push(DisassemblyOperand {
            name: stringify!($operand),
            value: Box::new(unsafe { <$ty>::decode(buf, pc, width) })
          });
          pc += <$ty>::size(width);
        )*

        Disassembly {
          name: <$name>::NAME,
          width,
          operands,
          size: pc - start_pc,
        }
      }
    }

    impl From<$name> for $Instruction {
      fn from(v: $name) -> Self {
        Self::$name(v)
      }
    }

    impl Opcode for $name {
      const NAME: &'static str = to_snake_case_str!($name);
    }

    impl Decode for $name {
      type Operands = ($($ty),*);
      type Decoded = ($(<$ty as Operand>::Decoded),*);

      #[allow(unused_mut, unused_assignments)]
      #[inline]
      fn decode(bytecode: &[u8], mut pc: usize, width: Width) -> Self::Decoded {
        assert!(bytecode.len() > pc + Self::Operands::size(width));
        $(
          let $operand = unsafe { <$ty>::decode(bytecode, pc, width) };
          pc += <$ty>::size(width);
        )*
        ($($operand),*)
      }
    }

    impl Encode for $name {
      #[inline]
      fn encode(&self, buf: &mut Vec<u8>, force_max_width: bool) {
        let $name { $($operand),* } = self;

        let prefix = if force_max_width {
          Width::Quad
        } else {
          Width::Single $( | <$ty as Operand>::width(*$operand) )*
        };
        prefix.encode(buf);
        buf.push(_Kind::$name as u8);
        $(
          <$ty as Operand>::encode(buf, *$operand, force_max_width);
        )*
      }
    }

    impl_encode_into_for_jump!($(:$jump)? $name ($($operand : $ty),*));

    impl private::Sealed for $name {}
  }
}

macro_rules! instruction_dispatch {
  ($Handler:ident; $name:ident, ()) => {
    paste! {
      fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        bc: &mut Vec<u8>,
        pc: &mut usize,
        opcode: &mut u8,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        if cfg!(feature = "disassembly") {
          println!("{}", disassemble(bc, *pc));
        }

        *result = vm.[<op_ $name:snake>]();
        *pc += 1;
        *width = Width::Single;
        *opcode = bc[*pc];
      }
    }
  };
  ($Handler:ident; $name:ident, ($( $operand:ident : $ty:ty ),+)) => {
    paste! {
      fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        bc: &mut Vec<u8>,
        pc: &mut usize,
        opcode: &mut u8,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        if cfg!(feature = "disassembly") {
          println!("{}", disassemble(bc, *pc));
        }

        let ($($operand),*) = <$name>::decode(bc, *pc + 1, *width);
        *result = vm.[<op_ $name:snake>]($($operand),*);
        *pc += 1 + <$name as Decode>::Operands::size(*width);
        *width = Width::Single;
        *opcode = bc[*pc];
      }
    }
  };
  ($Handler:ident; :jump $name:ident, ($( $operand:ident : $ty:ty ),+)) => {
    paste! {
      fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        bc: &mut Vec<u8>,
        pc: &mut usize,
        opcode: &mut u8,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        if cfg!(feature = "disassembly") {
          println!("{}", disassemble(bc, *pc));
        }

        let ($($operand),*) = <$name>::decode(bc, *pc + 1, *width);
        handle_jump(
          vm.[<op_ $name:snake>]($($operand),*),
          pc,
          <$name as Decode>::Operands::size(*width),
          result
        );
        *width = Width::Single;
        *opcode = bc[*pc];
      }
    }
  };
}

macro_rules! handler_method {
  (:jump $name:ident, ($( $operand:ident : $ty:ty ),*)) => {
    paste! {
      #[allow(unused_variables)]
      fn [<op_ $name:snake>](
        &mut self,
        $($operand : <$ty as Operand>::Decoded),*
      ) -> Result<ControlFlow, Self::Error> {
        unimplemented!()
      }
    }
  };
  ($name:ident, ($( $operand:ident : $ty:ty ),*)) => {
    paste! {
      #[allow(unused_variables)]
      fn [<op_ $name:snake>](
        &mut self,
        $($operand : <$ty as Operand>::Decoded),*
      ) -> Result<(), Self::Error>  {
        unimplemented!()
      }
    }
  };
}

macro_rules! instructions {
  (
    $Instruction:ident, $ops:ident,
    $Handler:ident, $run:ident,
    $Nop:ident, $Wide:ident, $ExtraWide:ident, $Suspend:ident,
    $disassemble:ident;
    $( $name:ident $(:$jump:ident)? ($( $operand:ident : $ty:ty ),*) $(= $index:literal)? ),* $(,)?
  ) => {

    #[repr(u8)]
    enum _Kind {
      $Nop = 0,
      $( $name $( = $index )? ),*,
      $Suspend = 255,
    }

    #[derive(Debug, Clone)]
    #[repr(u8)]
    pub enum $Instruction {
      $Nop($Nop) = _Kind::$Nop as u8,
      $( $name($name) = _Kind::$name as u8 ),*,
      $Suspend($Suspend) = _Kind::$Suspend as u8,
    }

    impl Encode for $Instruction {
      fn encode(&self, buf: &mut Vec<u8>, force_max_width: bool) {
        match self {
          $Instruction::$Nop(v) => v.encode(buf, force_max_width),
          $( $Instruction::$name(v) => v.encode(buf, force_max_width), )*
          $Instruction::$Suspend(v) => v.encode(buf, force_max_width),
        }
      }
    }

    impl $Instruction {
      pub const fn names() -> &'static [&'static str] {
        &[
          <$Nop>::NAME,
          $( <$name>::NAME ),*
        ]
      }
    }

    pub mod $ops {
      #![allow(non_upper_case_globals)]
      use super::_Kind;

      pub const $Nop: u8 = _Kind::$Nop as u8;
      pub const $Wide: u8 = 0x01;
      pub const $ExtraWide: u8 = 0x02;

      $( pub const $name: u8 = _Kind::$name as u8; )*

      pub const $Suspend: u8 = _Kind::$Suspend as u8;
    }

    instruction_base!(
      $Instruction, $Handler;
      $Nop () = 0
    );

    fn op_nop<H: $Handler>(
      _: &mut H,
      bc: &mut Vec<u8>,
      pc: &mut usize,
      opcode: &mut u8,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      *pc += 1;
      *width = Width::Single;
      *opcode = bc[*pc];
    }

    fn op_wide<H: $Handler>(
      _: &mut H,
      bc: &mut Vec<u8>,
      pc: &mut usize,
      opcode: &mut u8,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      *pc += 1;
      *width = Width::Double;
      *opcode = bc[*pc];
    }

    fn op_extra_wide<H: $Handler>(
      _: &mut H,
      bc: &mut Vec<u8>,
      pc: &mut usize,
      opcode: &mut u8,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      *pc += 1;
      *width = Width::Quad;
      *opcode = bc[*pc];
    }

    instruction_base!(
      $Instruction, $Handler;
      $Suspend () = 255
    );

    $(
      instruction_base!(
        $Instruction, $Handler;
        $(:$jump)? $name ($( $operand : $ty )*) $(= $index)?
      );

      instruction_dispatch!($Handler; $(:$jump)? $name, ($($operand : ty),*));
    )*

    impl private::Sealed for Instruction {}

    paste! {
      pub trait $Handler {
        type Error;

        $( handler_method!($(:$jump)? $name, ($($operand : $ty),*)); )*
      }
    }

    #[inline(never)]
    pub fn $run<H: $Handler>(vm: &mut H, bc: &mut Vec<u8>, pc: &mut usize) -> Result<(), H::Error> {
      let opcode = &mut (bc[*pc].clone());
      let width = &mut Width::Single;
      let mut result = Ok(());
      while result.is_ok() {
        let result = &mut result;
        match *opcode {
          ops::$Nop => op_nop(vm, bc, pc, opcode, width, result),
          ops::$Wide => op_wide(vm, bc, pc, opcode, width, result),
          ops::$ExtraWide => op_extra_wide(vm, bc, pc, opcode, width, result),
          $(
            ops::$name => paste!([<op_ $name:snake>])(vm, bc, pc, opcode, width, result),
          )*
          ops::$Suspend => break,
          _ => panic!("malformed bytecode: invalid opcode {}", *opcode),
        }
      }
      result
    }

    pub fn $disassemble(buf: &[u8], offset: usize) -> Disassembly {
      let (offset, width) = match buf[offset] {
        ops::$Wide => (offset + 1, Width::Double),
        ops::$ExtraWide => (offset + 1, Width::Quad),
        _ => (offset, Width::Single),
      };

      match buf[offset] {
        ops::$Nop => <$Nop>::disassemble(buf, offset, width),
        $(
          ops::$name => <$name>::disassemble(buf, offset, width),
        )*
        ops::$Suspend => <$Suspend>::disassemble(buf, offset, width),
        opcode => panic!("malformed bytecode: invalid opcode 0x{opcode:02x}"),
      }
    }

    fn decode_size(buf: &[u8]) -> usize {
      let (offset, width) = match buf[0] {
        ops::$Wide => (1, Width::Double),
        ops::$ExtraWide => (1, Width::Quad),
        _ => (0, Width::Single),
      };

      match buf[offset] {
        ops::$Nop => offset + 1,
        $(
          ops::$name => offset + 1 + <$name as Decode>::Operands::size(width),
        )*
        ops::$Suspend => offset + 1,
        opcode => panic!("malformed bytecode: invalid opcode 0x{opcode:02x}"),
      }
    }
  }
}
