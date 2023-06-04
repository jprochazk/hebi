macro_rules! read_opcode {
  ($ip:ident, $end:ident) => {
    unsafe {
      if $ip >= $end {
        panic!(
          "unexpected end of bytecode stream (pc={})",
          ($ip as usize) - ($end as usize)
        );
      }
      let opcode = $ip.read();
      $ip = $ip.add(1);
      debug_assert!(
        $crate::bytecode::opcode::Opcode::try_from(opcode).is_ok(),
        "illegal instruction"
      );
      ::core::mem::transmute::<u8, Opcode>(opcode)
    }
  };
}

#[doc(hidden)]
#[inline]
pub unsafe fn __read_tuple<const N: usize, T: crate::bytecode::operands::Operand>(
  ip: *mut u8,
  width: crate::bytecode::operands::Width,
) -> T {
  let len = N * width as usize;
  let buf = &*std::ptr::slice_from_raw_parts(ip, len);
  T::decode(buf, width)
}

macro_rules! read_operands {
  ($T:ident, $ip:ident, $end:ident, $width:ident) => {{
    type Operands =
      <$crate::bytecode::opcode::symbolic::$T as $crate::bytecode::opcode::Operands>::Operands;
    const LENGTH: usize = <Operands as $crate::util::TupleLength>::LENGTH;
    if LENGTH > 0 {
      unsafe {
        if $ip >= $end {
          panic!(
            "unexpected end of bytecode stream (pc={})",
            ($ip as usize) - ($end as usize)
          );
        }
        let operands = $crate::vm::dispatch::macros::__read_tuple::<LENGTH, Operands>($ip, $width);
        $ip = $ip.add(LENGTH * ($width as usize));
        $width = Width::Normal;
        operands
      }
    } else {
      Operands::default()
    }
  }};
}

macro_rules! get_pc {
  ($ip:ident, $code:ident) => {
    ($ip as usize) - ($code.as_ptr() as *mut u8 as usize)
  };
}
