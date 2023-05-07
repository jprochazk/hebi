macro_rules! operands {
  ($T:ident, $code:ident, $pc:ident, $width:ident) => {{
    use $crate::bytecode::operands::Operand;
    type Operands =
      <$crate::bytecode::opcode::symbolic::$T as $crate::bytecode::opcode::Operands>::Operands;
    let buf = $code.get($pc + 1..).ok_or_else(|| Error::UnexpectedEnd)?;
    Operands::decode(buf, $width)
  }};
}

macro_rules! size_of_operands {
  ($T:ident) => {{
    type Operands =
      <$crate::bytecode::opcode::symbolic::$T as $crate::bytecode::opcode::Operands>::Operands;
    <Operands as $crate::util::TupleLength>::LENGTH
  }};
}
