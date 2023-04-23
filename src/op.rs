pub mod emit;

// TODO: more `long` instructions which store operands as constants
// TODO: document the rest

#[repr(u8)]
pub enum Instruction {
  Noop,

  /// Load a constant into the accumulator.
  LoadConstant(Constant),
  /// Load a register into the accumulator.
  LoadRegister(Register),
  /// Store the accumulator into a register.
  StoreRegister(Register),

  /// Load an upvalue into the accumulator.
  LoadUpvalue(Upvalue),
  /// Store the accumulator into an upvalue.
  StoreUpvalue(Upvalue),

  /// Load a module variable into the accumulator.
  LoadModuleVar(ModuleVar),
  /// Store the accumulator into a module variable.
  StoreModuleVar(ModuleVar),

  /// Load a global into the accumulator.
  LoadGlobal(Constant),
  /// Store the accumulator into a global.
  StoreGlobal(Constant),

  /// Load a field from the object in the accumulator into the accumulator.
  ///
  /// If the object does not have the field, panic.
  LoadField(Constant),
  /// Load an object field into the accumulator.
  ///
  /// The object is stored in the accumulator.
  ///
  /// If the object does not have the field, load `none` into the accumulator.
  MaybeLoadField(Constant),
  /// Store the accumulator into an object field.
  ///
  /// The object is stored in a register.
  StoreField(Register, Constant),

  LoadIndex(Register),
  MaybeLoadIndex(Register),
  StoreIndex(Register),

  LoadSelf,
  LoadSuper,

  LoadNone,
  LoadTrue,
  LoadFalse,
  LoadSmi(i16),

  MakeFn(Constant),
  CloseReg(Register, Upvalue),
  CloseSlot(Upvalue, Upvalue),

  MakeClass(Register, Constant),

  Jump(Offset),
  LongJump(Constant),
  Loop(Offset),
  LongLoop(Constant),
  JumpIfFalse(Offset),
  LongJumpIfFalse(Constant),

  Add(Register),
  Sub(Register),
  Multiplty(Register),
  Divide(Register),
  Remainder(Register),
  Power(Register),
  Inverse,
  Not,

  CompareEqual(Register),
  CompareNotEqual(Register),
  CompareGreaterThan(Register),
  CompareGreaterEqual(Register),
  CompareLessThan(Register),
  CompareLessEqual(Register),

  Is(Register),
  In(Register),

  Print(Register, u8),
  PrintList(Constant),

  Call(Register, u8),

  Return,
  Yield = u8::MAX,
}

static_assert_size!(Instruction, u32);

pub struct Offset(u16);

pub struct Constant(u16);

pub struct Register(u8);

pub struct Upvalue(u8);

pub struct ModuleVar(u16);
