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
    $name:ident ($( $operand:ident : $ty:ident ),*)
  ) => {
    #[derive(Debug, Clone)]
    pub struct $name {
      $( pub $operand : <$ty as Operand>::Decoded ),*
    }
  };
}

macro_rules! impl_encode_into_for_jump {
  ( :jump $name:ident ($( $operand:ident : $ty:ident ),*) ) => {
    impl EncodeInto for $name {
      #[inline]
      #[allow(unused_assignments)]
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
      }
    }
  };
  ( $(:$flag:ident)? $name:ident ($( $operand:ident : $ty:ident ),*) ) => {};
}

macro_rules! disassembly_operand_kind {
  (Const) => {
    DisassemblyOperandKind::Const
  };
  (Reg) => {
    DisassemblyOperandKind::Reg
  };
  ($ty:ident) => {
    DisassemblyOperandKind::Simple
  };
}

macro_rules! instruction_base {
  (
    $Instruction:ident, $Handler:ident;
    $(:$flag:ident)? $name:ident ($( $operand:ident : $ty:ident )*) $(= $index:literal)?
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
            value: Box::new(unsafe { <$ty>::decode(buf, pc, width) }),
            kind: disassembly_operand_kind!($ty),
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

    impl_encode_into_for_jump!($(:$flag)? $name ($($operand : $ty),*));

    impl private::Sealed for $name {}
  }
}

macro_rules! instruction_dispatch {
  ($Handler:ident; $name:ident, ()) => {
    paste! {
      #[allow(clippy::ptr_arg)]
      unsafe fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        _: std::ptr::NonNull<[u8]>,
        pc: std::ptr::NonNull<usize>,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        *result = vm.[<op_ $name:snake>]();
        *pc.as_ptr() += 1;
        *width = Width::Single;
      }
    }
  };
  ($Handler:ident; $name:ident, ($( $operand:ident : $ty:ident ),+)) => {
    paste! {
      #[allow(clippy::ptr_arg)]
      unsafe fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        bc: std::ptr::NonNull<[u8]>,
        pc: std::ptr::NonNull<usize>,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        let ($($operand),*) = <$name>::decode(bc.as_ref(), (*pc.as_ptr()) + 1, *width);
        *result = vm.[<op_ $name:snake>]($($operand),*);
        *pc.as_ptr() += 1 + <$name as Decode>::Operands::size(*width);
        *width = Width::Single;
      }
    }
  };
  ($Handler:ident; :jump $name:ident, ($( $operand:ident : $ty:ident ),+)) => {
    paste! {
      #[allow(clippy::ptr_arg)]
      unsafe fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        bc: std::ptr::NonNull<[u8]>,
        pc: std::ptr::NonNull<usize>,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        let ($($operand),*) = <$name>::decode(bc.as_ref(), (*pc.as_ptr()) + 1, *width);
        handle_jump(
          vm.[<op_ $name:snake>]($($operand),*),
          pc,
          <$name as Decode>::Operands::size(*width),
          result
        );
        *width = Width::Single;
      }
    }
  };
  ($Handler:ident; :call $name:ident, ()) => {
    paste! {
      #[allow(clippy::ptr_arg)]
      unsafe fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        _: std::ptr::NonNull<[u8]>,
        pc: std::ptr::NonNull<usize>,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        // VM sets `PC`
        *result = vm.[<op_ $name:snake>](
          *pc.as_ptr() + 1 + <$name as Decode>::Operands::size(*width)
        );
        *width = Width::Single;
      }
    }
  };
  ($Handler:ident; :call $name:ident, ($( $operand:ident : $ty:ident ),+)) => {
    paste! {
      #[allow(clippy::ptr_arg)]
      unsafe fn [<op_ $name:snake>]<H: $Handler>(
        vm: &mut H,
        bc: std::ptr::NonNull<[u8]>,
        pc: std::ptr::NonNull<usize>,
        width: &mut Width,
        result: &mut Result<(), H::Error>,
      ) {
        let ($($operand),*) = <$name>::decode(bc.as_ref(), (*pc.as_ptr()) + 1, *width);
        // VM sets `PC`
        *result = vm.[<op_ $name:snake>](
          $($operand,)*
          *pc.as_ptr() + 1 + <$name as Decode>::Operands::size(*width)
        );
        *width = Width::Single;
      }
    }
  };
}

macro_rules! handler_method {
  (:jump $(#[$meta:meta])* $name:ident, ($( $operand:ident : $ty:ident ),*)) => {
    paste! {
      #[allow(unused_variables)]
      $(#[$meta])*
      fn [<op_ $name:snake>](
        &mut self,
        $($operand : <$ty as Operand>::Decoded),*
      ) -> Result<ControlFlow, Self::Error>;
    }
  };
  (:call $(#[$meta:meta])* $name:ident, ($( $operand:ident : $ty:ident ),*)) => {
    paste! {
      #[allow(unused_variables)]
      $(#[$meta])*
      fn [<op_ $name:snake>](
        &mut self,
        $($operand : <$ty as Operand>::Decoded,)*
        return_address: usize,
      ) -> Result<(), Self::Error>;
    }
  };
  ($(#[$meta:meta])* $name:ident, ($( $operand:ident : $ty:ident ),*)) => {
    paste! {
      #[allow(unused_variables)]
      $(#[$meta])*
      fn [<op_ $name:snake>](
        &mut self,
        $($operand : <$ty as Operand>::Decoded),*
      ) -> Result<(), Self::Error>;
    }
  };
}

macro_rules! update_register {
  ($map:ident, $inner:ident, $operand:ident : Reg) => {
    $inner.$operand = $map[&$inner.$operand]
  };
  ($map:ident, $inner:ident, $operand:ident : $ty:ident) => {};
}

macro_rules! instructions {
  (
    $Instruction:ident, $ops:ident,
    $Handler:ident, $run:ident,
    $Nop:ident, $Wide:ident, $ExtraWide:ident,
    $Ret:ident, $Suspend:ident,
    $disassemble:ident, $update_registers:ident;
    $(
      $(#[$meta:meta])*
      $name:ident $(:$flag:ident)? ($( $operand:ident : $ty:ident ),*) $(= $index:literal)?
    ),* $(,)?
  ) => {

    #[repr(u8)]
    enum _Kind {
      $Nop = 0,
      $( $name $( = $index )? ),*,
      $Ret = 254,
      $Suspend = 255,
    }

    #[derive(Debug, Clone)]
    #[repr(u8)]
    pub enum $Instruction {
      /// Do nothing.
      $Nop($Nop) = _Kind::$Nop as u8,
      $( $(#[$meta])* $name($name) = _Kind::$name as u8 ,)*
      $Ret($Ret) = _Kind::$Ret as u8,
      /// Suspend the dispatch loop.
      $Suspend($Suspend) = _Kind::$Suspend as u8,
    }

    impl Encode for $Instruction {
      fn encode(&self, buf: &mut Vec<u8>, force_max_width: bool) {
        match self {
          $Instruction::$Nop(v) => v.encode(buf, force_max_width),
          $( $Instruction::$name(v) => v.encode(buf, force_max_width), )*
          $Instruction::$Ret(v) => v.encode(buf, force_max_width),
          $Instruction::$Suspend(v) => v.encode(buf, force_max_width),
        }
      }
    }

    /* impl $Instruction {
      pub const fn names() -> &'static [&'static str] {
        &[
          <$Nop>::NAME,
          $( <$name>::NAME ),*
        ]
      }
    } */

    pub mod $ops {
      #![allow(non_upper_case_globals)]
      use super::_Kind;

      /// Do nothing.
      pub const $Nop: u8 = _Kind::$Nop as u8;
      /// Variable-width encoding prefix marker.
      ///
      /// Scales variable-width operands to 2x (1 byte -> 2 bytes).
      pub const $Wide: u8 = 0x01;
      /// Variable-width encoding prefix marker.
      ///
      /// Scales variable-width operands to 4x (1 byte -> 4 bytes).
      pub const $ExtraWide: u8 = 0x02;

      $( $(#[$meta])* pub const $name: u8 = _Kind::$name as u8; )*

      pub const $Ret: u8 = _Kind::$Ret as u8;

      /// Suspend the dispatch loop.
      pub const $Suspend: u8 = _Kind::$Suspend as u8;
    }

    instruction_base!(
      $Instruction, $Handler;
      $Nop () = 0
    );

    #[allow(clippy::ptr_arg)]
    unsafe fn op_nop<H: $Handler>(
      _: &mut H,
      _: std::ptr::NonNull<[u8]>,
      pc: std::ptr::NonNull<usize>,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      *pc.as_ptr() += 1;
      *width = Width::Single;
    }

    #[allow(clippy::ptr_arg)]
    unsafe fn op_wide<H: $Handler>(
      _: &mut H,
      _: std::ptr::NonNull<[u8]>,
      pc: std::ptr::NonNull<usize>,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      *pc.as_ptr() += 1;
      *width = Width::Double;
    }

    #[allow(clippy::ptr_arg)]
    unsafe fn op_extra_wide<H: $Handler>(
      _: &mut H,
      _: std::ptr::NonNull<[u8]>,
      pc: std::ptr::NonNull<usize>,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      *pc.as_ptr() += 1;
      *width = Width::Quad;
    }

    #[allow(clippy::ptr_arg)]
    unsafe fn op_ret<H: Handler>(
      vm: &mut H,
      _: std::ptr::NonNull<[u8]>,
      _: std::ptr::NonNull<usize>,
      width: &mut Width,
      result: &mut Result<(), H::Error>,
    ) {
      *result = vm.op_ret();
      *width = Width::Single;
    }

    instruction_base!(
      $Instruction, $Handler;
      $Ret () = 254
    );

    instruction_base!(
      $Instruction, $Handler;
      $Suspend () = 255
    );

    $(
      instruction_base!(
        $Instruction, $Handler;
        $(:$flag)? $name ($( $operand : $ty )*) $(= $index)?
      );

      instruction_dispatch!($Handler; $(:$flag)? $name, ($($operand : ty),*));
    )*

    impl private::Sealed for Instruction {}

    paste! {
      pub trait $Handler {
        type Error;

        $( handler_method!($(:$flag)? $(#[$meta])* $name, ($($operand : $ty),*)); )*

        fn op_ret(&mut self) -> Result<(), Self::Error>;
      }
    }

    /// Execute bytecode in `bc` starting at `pc`.
    ///
    /// Handles decoding variable-width operands,
    /// and delegates implementation to `vm`.
    ///
    /// # Safety
    ///
    /// `bc` and `pc` must be valid pointers, as defined in https://doc.rust-lang.org/std/ptr/index.html#safety.
    #[inline(never)]
    pub unsafe fn $run<H: $Handler>(vm: &mut H, bc: std::ptr::NonNull<[u8]>, pc: std::ptr::NonNull<usize>) -> Result<(), H::Error> {
      #[inline]
      unsafe fn read_at(a: std::ptr::NonNull<[u8]>, i: std::ptr::NonNull<usize>) -> u8 {
        let i = std::ptr::read(i.as_ptr());
        std::ptr::read(a.cast::<u8>().as_ptr().add(i))
      }

      let mut opcode = read_at(bc, pc);
      let mut width = Width::Single;
      let mut result = Ok(());
      while result.is_ok() {
        let width = &mut width;
        let result = &mut result;
        match opcode {
          ops::$Nop => op_nop(vm, bc, pc, width, result),
          ops::$Wide => op_wide(vm, bc, pc, width, result),
          ops::$ExtraWide => op_extra_wide(vm, bc, pc, width, result),
          $(
            ops::$name => paste!([<op_ $name:snake>])(vm, bc, pc, width, result),
          )*
          ops::$Ret => op_ret(vm, bc, pc, width, result),
          ops::$Suspend => break,
          _ => panic!("malformed bytecode: invalid opcode {}", opcode),
        }
        opcode = read_at(bc, pc);
      }
      result
    }

    pub fn $disassemble(buf: &[u8], offset: usize) -> (usize, Disassembly) {
      let (offset, width) = match buf[offset] {
        ops::$Wide => (offset + 1, Width::Double),
        ops::$ExtraWide => (offset + 1, Width::Quad),
        _ => (offset, Width::Single),
      };

      let dis = match buf[offset] {
        ops::$Nop => <$Nop>::disassemble(buf, offset, width),
        $(
          ops::$name => <$name>::disassemble(buf, offset, width),
        )*
        ops::$Ret => <$Ret>::disassemble(buf, offset, width),
        ops::$Suspend => <$Suspend>::disassemble(buf, offset, width),
        opcode => panic!("malformed bytecode: invalid opcode 0x{opcode:02x}"),
      };
      (dis.size(), dis)
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
        ops::$Ret => offset + 1,
        ops::$Suspend => offset + 1,
        opcode => panic!("malformed bytecode: invalid opcode 0x{opcode:02x}"),
      }
    }

    #[allow(unused_variables)]
    pub fn $update_registers(instruction: &mut $Instruction, map: &indexmap::IndexMap<u32, u32>) {
      match instruction {
        $Instruction::$Nop(_) => {}
        $(
          $Instruction::$name(inner) => {
            $( update_register!(map, inner, $operand : $ty); )*
          },
        )*
        $Instruction::$Ret(_) => {}
        $Instruction::$Suspend(_) => {}
      }
    }
  }
}
