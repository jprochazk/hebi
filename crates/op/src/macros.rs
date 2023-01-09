macro_rules! define_bytecode {
  (
    ($handler:ident, $run:ident, $builder:ident, $error:ident, $op:ident)
    $($tail:tt)*
  ) => {
    pub trait $handler {
      type $error;
      define_bytecode!(__handler_method $error; $($tail)*);
    }

    mod $op {
      pub const __nop: u8 = 0;
      pub const __wide: u8 = 1;
      pub const __xwide: u8 = 2;
      define_bytecode!(__op_const __xwide; $($tail)*);
      pub const __halt: u8 = 255;

      const fn _check_num_opcodes() {
        let num_opcodes = define_bytecode!(__count $($tail)*);
        let max_opcodes = 255 - 4;
        if num_opcodes >= max_opcodes {
          panic!("too many opcodes");
        }
      }
      const _: () = _check_num_opcodes();
    }

    #[inline]
    fn op_nop<H: $handler>(_: &mut H, bc: &mut BytecodeArray, pc: &mut usize, opcode: &mut u8, operand_size: &mut Width, _: &mut Result<(), H::Error>) {
      *pc += 1;
      *operand_size = Width::_1;
      *opcode = bc.fetch(*pc);
    }

    #[inline]
    fn op_wide<H: $handler>(_: &mut H, bc: &mut BytecodeArray, pc: &mut usize, opcode: &mut u8, operand_size: &mut Width, _: &mut Result<(), H::Error>) {
      *pc += 1;
      *operand_size = Width::_2;
      *opcode = bc.fetch(*pc);
    }

    #[inline]
    fn op_xwide<H: $handler>(_: &mut H, bc: &mut BytecodeArray, pc: &mut usize, opcode: &mut u8, operand_size: &mut Width, _: &mut Result<(), H::Error>) {
      *pc += 1;
      *operand_size = Width::_4;
      *opcode = bc.fetch(*pc);
    }

    define_bytecode!(__dispatch_handler $handler; $($tail)*);

    #[inline(never)]
    pub fn $run<H: $handler>(vm: &mut H, bc: &mut BytecodeArray) -> Result<(), H::Error> {
      let pc = &mut 0;
      let opcode = &mut bc.fetch(*pc);
      let operand_size = &mut Width::_1;
      let mut result = Ok(());
      while !result.is_err() {
        let result = &mut result;
        define_bytecode!(__dispatch_match $op (vm, bc, pc, opcode, operand_size, result); $($tail)*);
      }
      result
    }
  };

  (__handler_method $error:ident;) => {};
  (__handler_method $error:ident; $(#[$meta:meta])* $name:ident $(<$arg:ident>)* , $($tail:tt)*) => {
    paste! {
      $(#[$meta])*
      fn [<op_ $name>](&mut self, $($arg : u32),*) -> Result<(), Self::$error>;
    }
    define_bytecode!(__handler_method $error; $($tail)*);
  };

  (__op_const $prev:ident;) => {};
  (__op_const $prev:ident; $(#[$meta:meta])* $name:ident $(<$arg:ident>)* , $($tail:tt)*) => {
    pub const $name : u8 = $prev + 1;
    define_bytecode!(__op_const $name; $($tail)*);
  };

  (__count) => { 0 };
  (__count $(#[$meta:meta])* $name:ident $(<$arg:ident>)* , $($tail:tt)*) => {
    1 + define_bytecode!(__count $($tail)*)
  };

  (__count_args) => { 0 };
  (__count_args $arg:ident, $($tail:tt)*) => {
    1 + define_bytecode!(__count_args $($tail)*)
  };

  (__dispatch_handler $handler:ident;) => {};
  (__dispatch_handler $handler:ident; $(#[$meta:meta])* $name:ident $(<$arg:ident>)*, $($tail:tt)*) => {
    paste! {
      #[inline]
      fn [<op_ $name>]<H: $handler>(vm: &mut H, bc: &mut BytecodeArray, pc: &mut usize, opcode: &mut u8, operand_size: &mut Width, result: &mut Result<(), H::Error>) {
        const ARGC: usize = define_bytecode!(__count_args $($arg,)*);
        let [$($arg),*] = bc.get_args::<ARGC>(*opcode, *pc, *operand_size);
        *result = vm.[<op_ $name>]($($arg),*);

        *pc += 1 + ARGC * (*operand_size) as usize;
        *operand_size = Width::_1;
        *opcode = bc.fetch(*pc);
      }
    }
    define_bytecode!(__dispatch_handler $handler; $($tail)*);
  };

  (
    __dispatch_match $op:ident ($vm:ident, $bc:ident, $pc:ident, $opcode:ident, $operand_size:ident, $result:ident);
    $( $(#[$meta:meta])* $name:ident $(<$arg:ident>)* , )*
  ) => {
    paste! {
      match *$opcode {
        $op::__nop => op_nop($vm, $bc, $pc, $opcode, $operand_size, $result),
        $op::__wide => op_wide($vm, $bc, $pc, $opcode, $operand_size, $result),
        $op::__xwide => op_xwide($vm, $bc, $pc, $opcode, $operand_size, $result),

        $(
          $op::$name => [<op_ $name>]($vm, $bc, $pc, $opcode, $operand_size, $result),
        )*

        $op::__halt => break,
        _ => unreachable!("malformed bytecode: invalid opcode {}", $opcode),
      }
    }
  };
}
