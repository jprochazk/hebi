pub enum Jump {
  /// Go to `offset`.
  Goto { offset: u32 },
  /// Skip this jump instruction.
  Skip,
}

pub trait Handler {
  type Error;
  /// Load a constant into the accumulator.
  fn op_load_const(&mut self, slot: u32) -> Result<(), Self::Error>;

  /// Load a register into the accumulator.
  fn op_load_reg(&mut self, reg: u32) -> Result<(), Self::Error>;

  /// Load the accumulator into a register.
  fn op_store_reg(&mut self, reg: u32) -> Result<(), Self::Error>;

  /// Jump to the specified offset.
  fn op_jump(&mut self, offset: u32) -> Result<Jump, Self::Error>;

  /// Jump to the specified offset if the value in the accumulator is falsey.
  fn op_jump_if_false(&mut self, offset: u32) -> Result<Jump, Self::Error>;

  /// Subtract `b` from `a` and store the result in `dest`.
  fn op_sub(&mut self, lhs: u32) -> Result<(), Self::Error>;

  /// Print a value in a register.
  fn op_print(&mut self, reg: u32) -> Result<(), Self::Error>;

  /// Push an integer into the accumulator.
  fn op_push_small_int(&mut self, value: i32) -> Result<(), Self::Error>;

  /// Create an empty list and store it in the accumulator.
  fn op_create_empty_list(&mut self, _: ()) -> Result<(), Self::Error>;

  /// Push a value from the accumulator into the list in register `list`.
  fn op_list_push(&mut self, list: u32) -> Result<(), Self::Error>;

  /// Return from the current function call.
  fn op_ret(&mut self, _: ()) -> Result<(), Self::Error>;
}
