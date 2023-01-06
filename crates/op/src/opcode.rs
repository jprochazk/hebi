use crate::u24::u24;

pub struct Chunk<Value> {
  pub bytecode: Vec<Opcode>,
  pub const_pool: Vec<Value>,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Opcode {
  LoadConst { index: u24 },
  LoadLocal { index: u24 },
  StoreLocal { index: u24 },
  LoadCapture { index: u24 },
  StoreCapture { index: u24 },
  LoadGlobal { index: u24 },
  StoreGlobal { index: u24 },
  LoadField,
  StoreField,

  CreateList { index: u24 },
  CreateDict { index: u24 },
  CreateClosure { index: u24 },
  CreateClass { index: u24 },

  Call,

  Pop { n: u24 },
  Jump { offset: u24 },
  JumpIfFalse { offset: u24 },
  Yield,
  Return,

  Add,
  Subtract,
  Multiply,
  Divide,
  Remainder,
  Power,

  AddAssign,
  SubtractAssign,
  MultiplyAssign,
  DivideAssign,
  RemainderAssign,
  PowerAssign,

  Negate,
  Not,
  Equal,
  LesserThan,
  GreaterThan,
  LesserEqual,
  GreaterEqual,
}

static_assertions::assert_eq_size!(Opcode, u32);
