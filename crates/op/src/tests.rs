use super::*;
// Things to test:

// 1. bytecode emit + disassembly
// 2. `run` loop
//   - jumps
//   - suspend
//   - errors

// write snapshots for all of the above

#[derive(Clone, Hash, PartialEq, Eq)]
enum Value {
  String(String),
  Number(u64),
  Bool(bool),
}

impl std::fmt::Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Value::String(v) => write!(f, "\"{v}\""),
      Value::Number(v) => write!(f, "{v}"),
      Value::Bool(v) => write!(f, "{v}"),
    }
  }
}

type Builder = BytecodeBuilder<Value>;

macro_rules! check {
  ($chunk:ident) => {{
    insta::assert_snapshot!($chunk.disassemble());
  }};
}

#[test]
fn test_builder() {
  let mut b = Builder::new("test");

  let [start, end] = b.labels(["start", "end"]);

  b.constant(Value::String("test".into()));
  b.constant(Value::Number(123_456_789));
  b.constant(Value::Bool(true));

  b.finish_label(start);
  b.op_nop();
  b.op_load_const(0u32);
  b.op_load_const(u8::MAX as u32 + 1);
  b.op_load_const(u16::MAX as u32 + 1);
  b.op_load_reg(0);
  b.op_load_reg(u8::MAX as u32 + 1);
  b.op_load_reg(u16::MAX as u32 + 1);
  b.op_store_reg(0);
  b.op_store_reg(u8::MAX as u32 + 1);
  b.op_store_reg(u16::MAX as u32 + 1);
  b.op_jump(start);
  b.op_jump(end);
  b.op_jump_if_false(start);
  b.op_jump_if_false(end);
  b.op_ret();
  b.finish_label(end);
  b.op_suspend();

  let chunk = b.build();
  check!(chunk);
}
