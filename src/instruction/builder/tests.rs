use super::*;
use crate::instruction::opcodes::symbolic::*;
use crate::instruction::opcodes::{self as op, Opcode};

#[rustfmt::skip]
#[test]
fn basic_emit() {
  let mut builder = BytecodeBuilder::new();

  builder.emit(LoadSmi { value: 10 });
  builder.emit(Store {
    register: op::Register(0),
  });
  builder.emit(LoadSmi { value: 5 });
  builder.emit(Add {
    rhs: op::Register(0),
  });
  builder.emit(Print);

  let (bytecode, _) = builder.finish();

  assert_eq!(
    bytecode,
    [
      Opcode::LoadSmi as u8, 10i8.to_le_bytes()[0],
      Opcode::Store as u8, /*register*/ 0,
      Opcode::LoadSmi as u8, 5i8.to_le_bytes()[0],
      Opcode::Add as u8, /*rhs*/ 0,
      Opcode::Print as u8,
    ],
  );
}

#[rustfmt::skip]
#[test]
fn emit_constant() {
  let mut builder = BytecodeBuilder::new();

  let a = builder.constant_pool_builder().insert(NonNaNFloat::from(10.0));
  let b = builder.constant_pool_builder().insert(NonNaNFloat::from(5.0));
  let c = builder.constant_pool_builder().insert(NonNaNFloat::from(10.0));
  builder.emit(LoadConst { index: a });
  builder.emit(LoadConst { index: b });
  builder.emit(LoadConst { index: c });

  let (bytecode, constant) = builder.finish();

  assert_eq!(
    bytecode,
    [
      Opcode::LoadConst as u8, /*index*/ 0,
      Opcode::LoadConst as u8, /*index*/ 1,
      Opcode::LoadConst as u8, /*index*/ 0,
    ],
  );

  assert_eq!(constant.len(), 2);
  assert_eq!(constant[0].as_float().unwrap().value(), 10.0);
  assert_eq!(constant[1].as_float().unwrap().value(), 5.0);
}

#[rustfmt::skip]
#[test]
fn emit_forward_jump_8bit() {
  let mut builder = BytecodeBuilder::new();

  let test = builder.label("test");
  builder.emit(Nop);
  builder.emit_jump(&test);
  builder.emit(Nop);
  builder.emit(Nop);
  builder.emit(Nop);
  builder.emit(Nop);
  builder.emit(Nop);
  builder.emit(Nop);
  builder.bind_label(test);
  builder.emit(Ret);

  let (bytecode, _) = builder.finish();

  assert_eq!(
    bytecode,
    [ 
      Opcode::Nop as u8,
      Opcode::Jump as u8, /*offset*/ 8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Ret as u8,
    ],
  );
}

#[test]
fn emit_forward_jump_8bit_overflow() {
  let mut builder = BytecodeBuilder::new();

  let test = builder.label("test");
  builder.emit_jump(&test);
  for _ in 0..(256 - 2) {
    builder.emit(Nop);
  }
  builder.bind_label(test);
  builder.emit(Ret);

  let (bytecode, constants) = builder.finish();

  assert_eq!(bytecode[..2], [Opcode::JumpConst as u8, /* index */ 0],);
  assert!(bytecode[2..256].iter().all(|v| *v == Opcode::Nop as u8));
  assert_eq!(bytecode[256..], [Opcode::Ret as u8]);
  assert_eq!(constants.last().unwrap().as_offset().unwrap().0, 256);
}

#[test]
fn emit_forward_jump_16bit() {
  let mut builder = BytecodeBuilder::new();

  // fill constant pool to force 16-bit index
  for _ in 0..u8::MAX as u16 + 1 {
    builder.constant_pool_builder().insert(op::Offset(0));
  }

  let test = builder.label("test");
  builder.emit_jump(&test);

  let jump_len = 4; // jump instruction will use 4 bytes
  let num_nops = 256 - jump_len; // fill the rest of 8bit offset with nops
  for _ in 0..num_nops {
    builder.emit(Nop);
  }
  let jump_target = builder.bytecode.len() as u16;
  builder.bind_label(test);
  builder.emit(Ret);

  let (bytecode, _) = builder.finish();

  assert_eq!(
    bytecode[..jump_len],
    [
      Opcode::Wide16 as u8,
      Opcode::Jump as u8,
      // offset:
      jump_target.to_le_bytes()[0],
      jump_target.to_le_bytes()[1],
    ],
  );
  assert!(bytecode[jump_len..jump_len + num_nops]
    .iter()
    .all(|v| *v == Opcode::Nop as u8));
  assert_eq!(bytecode[jump_len + num_nops..], [Opcode::Ret as u8]);
}

#[test]
fn emit_forward_jump_16bit_overflow() {
  let mut builder = BytecodeBuilder::new();

  // fill constant pool to force 16-bit index
  for _ in 0..u8::MAX as u16 + 1 {
    builder.constant_pool_builder().insert(op::Offset(0));
  }

  let test = builder.label("test");
  builder.emit_jump(&test);

  let jump_len = 4; // jump instruction will use 4 bytes
  let num_nops = 65536 - jump_len; // fill the rest of 16bit offset with nops
  for _ in 0..num_nops {
    builder.emit(Nop);
  }
  let jump_target = builder.bytecode.len() as u32;
  builder.bind_label(test);
  builder.emit(Ret);

  let (bytecode, constants) = builder.finish();

  assert_eq!(
    bytecode[..jump_len],
    [
      Opcode::Wide16 as u8,
      Opcode::JumpConst as u8,
      // index:
      (u8::MAX as u16 + 1).to_le_bytes()[0],
      (u8::MAX as u16 + 1).to_le_bytes()[1],
    ],
  );
  assert!(bytecode[jump_len..jump_len + num_nops]
    .iter()
    .all(|v| *v == Opcode::Nop as u8));
  assert_eq!(bytecode[jump_len + num_nops..], [Opcode::Ret as u8]);
  assert_eq!(
    constants.last().unwrap().as_offset().unwrap().0,
    jump_target
  );
}

#[test]
fn emit_forward_jump_32bit() {
  let mut builder = BytecodeBuilder::new();

  // fill constant pool to force 32-bit index
  for _ in 0..u16::MAX as u32 + 1 {
    builder.constant_pool_builder().insert(op::Offset(0));
  }

  let test = builder.label("test");
  builder.emit_jump(&test);

  let jump_len = 6; // jump instruction will use 6 bytes
  let num_nops = 65536 - jump_len; // fill the rest of 16bit offset with nops
  for _ in 0..num_nops {
    builder.emit(Nop);
  }
  let jump_target = builder.bytecode.len() as u32;
  builder.bind_label(test);
  builder.emit(Ret);

  let (bytecode, _) = builder.finish();

  assert_eq!(
    bytecode[..jump_len],
    [
      Opcode::Wide32 as u8,
      Opcode::Jump as u8,
      // offset:
      jump_target.to_le_bytes()[0],
      jump_target.to_le_bytes()[1],
      jump_target.to_le_bytes()[2],
      jump_target.to_le_bytes()[3],
    ],
  );
  assert!(bytecode[jump_len..jump_len + num_nops]
    .iter()
    .all(|v| *v == Opcode::Nop as u8));
  assert_eq!(bytecode[jump_len + num_nops..], [Opcode::Ret as u8]);
}
