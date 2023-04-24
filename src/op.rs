pub mod emit;

// TODO: bytecode stream
// encode instructions as
//   [opcode : u8] [operands : ...]
// instructions will have LONG variants for larger operands where necessary

// TODO: more `long` instructions which store operands as constants
// TODO: document the rest

#[repr(u8)]
pub enum Instruction {
  Noop,
  LoadConstant(Constant),
  LoadRegister(Register),
  StoreRegister(Register),
  LoadUpvalue(Upvalue),
  StoreUpvalue(Upvalue),
  LoadModuleVar(ModuleVar),
  StoreModuleVar(ModuleVar),
  LoadGlobal(Constant),
  StoreGlobal(Constant),
  LoadField(Constant),
  MaybeLoadField(Constant),
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

pub struct Offset(pub u16);

pub struct Constant(pub u16);

pub struct Register(pub u8);

pub struct Upvalue(pub u8);

pub struct ModuleVar(pub u16);
