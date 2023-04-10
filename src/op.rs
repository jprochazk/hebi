pub mod emit;

// TODO: more `long` instructions which store operands as constants
// TODO: document the rest

#[repr(u8)]
pub enum Op {
  Nop,

  /// Load a constant into the accumulator.
  Ldac(Const),
  /// Load a register into the accumulator.
  Ldar(Reg),
  /// Store the accumulator into a register.
  Star(Reg),

  /// Load an upvalue into the accumulator.
  Ldau(Upval),
  /// Store the accumulator into an upvalue.
  Stau(Upval),

  /// Load a module variable into the accumulator.
  Ldamv(Mvar),
  /// Store the accumulator into a module variable.
  Stamv(Mvar),

  /// Load a global into the accumulator.
  Ldag(Const),
  /// Store the accumulator into a global.
  Stag(Const),

  /// Load a field from the object in the accumulator into the accumulator.
  ///
  /// If the object does not have the field, panic.
  LdaField(Const),
  /// Load an object field into the accumulator.
  ///
  /// The object is stored in the accumulator.
  ///
  /// If the object does not have the field, load `none` into the accumulator.
  LdaFieldOpt(Const),
  /// Store the accumulator into an object field.
  ///
  /// The object is stored in a register.
  StaField(Reg, Const),

  LdaIndex(Reg),
  LdaIndexOpt(Reg),
  StaIndex(Reg),

  LdaSelf,
  LdaSuper,

  LdaNone,
  LdaTrue,
  LdaFalse,
  LdaSmi(i16),

  MakeFn(Const),
  CloseReg(Reg, Upval),
  CloseSlot(Upval, Upval),

  MakeClass(Reg, Const),

  Jmp(Offset),
  JmpL(Const),
  Loop(Offset),
  LoopL(Const),
  Jmpa(Offset),
  JmpaL(Const),

  Add(Reg),
  Sub(Reg),
  Mul(Reg),
  Div(Reg),
  Rem(Reg),
  Pow(Reg),
  Inv,
  Not,

  CmpEq(Reg),
  CmpNe(Reg),
  CmpGt(Reg),
  CmpGe(Reg),
  CmpLt(Reg),
  CmpLe(Reg),

  Is(Reg),
  In(Reg),

  Print(Reg, u8),
  PrintL(Const),

  Call(Reg, u8),

  Ret,
  Yield = u8::MAX,
}

static_assert_size!(Op, u32);

pub struct Offset(u16);

pub struct Const(u16);

pub struct Reg(u8);

pub struct Upval(u8);

pub struct Mvar(u16);
