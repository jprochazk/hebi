use super::*;
use crate::bytecode::disasm::Disassembly;

#[rustfmt::skip]
#[test]
fn basic_emit() {
  let mut builder = BytecodeBuilder::new();

  builder.emit(LoadSmi { value: op::Smi(10) }, 0..0);
  builder.emit(Store {
    register: op::Register(0),
  }, 0..0);
  builder.emit(LoadSmi { value: op::Smi(5) }, 0..0);
  builder.emit(Add {
    rhs: op::Register(0),
  }, 0..0);
  builder.emit(Print, 0..0);

  let (bytecode, constants) = builder.finish();

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

  assert_snapshot!(Disassembly::new(&bytecode, &constants, 0).to_string());
}

#[rustfmt::skip]
#[test]
fn emit_constant() {
  let mut builder = BytecodeBuilder::new();

  let a = builder.constant_pool_builder().insert(NonNaNFloat::from(10.0));
  let b = builder.constant_pool_builder().insert(NonNaNFloat::from(5.0));
  let c = builder.constant_pool_builder().insert(NonNaNFloat::from(10.0));
  builder.emit(LoadConst { index: a }, 0..0);
  builder.emit(LoadConst { index: b }, 0..0);
  builder.emit(LoadConst { index: c }, 0..0);

  let (bytecode, constants) = builder.finish();

  assert_eq!(
    bytecode,
    [
      Opcode::LoadConst as u8, /*index*/ 0,
      Opcode::LoadConst as u8, /*index*/ 1,
      Opcode::LoadConst as u8, /*index*/ 0,
    ],
  );

  assert_eq!(constants.len(), 2);
  assert_eq!(constants[0].as_float().unwrap().value(), 10.0);
  assert_eq!(constants[1].as_float().unwrap().value(), 5.0);
  
  assert_snapshot!(Disassembly::new(&bytecode, &constants, 0).to_string());
}

#[rustfmt::skip]
#[test]
fn emit_forward_jump_8bit() {
  let mut builder = BytecodeBuilder::new();

  let test = builder.label("test");
  builder.emit(Nop, 0..0);
  builder.emit_jump(&test, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.bind_label(test);
  builder.emit(Ret, 0..0);

  let (bytecode, constants ) = builder.finish();

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
  
  assert_snapshot!(Disassembly::new(&bytecode, &constants, 0).to_string());
}

#[test]
fn emit_forward_jump_8bit_overflow() {
  let mut builder = BytecodeBuilder::new();

  let test = builder.label("test");
  builder.emit_jump(&test, 0..0);
  for _ in 0..(256 - 2) {
    builder.emit(Nop, 0..0);
  }
  builder.bind_label(test);
  builder.emit(Ret, 0..0);

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
  builder.emit_jump(&test, 0..0);

  let jump_len = 4; // jump instruction will use 4 bytes
  let num_nops = 256 - jump_len; // fill the rest of 8bit offset with nops
  for _ in 0..num_nops {
    builder.emit(Nop, 0..0);
  }
  let jump_target = builder.bytecode.len() as u16;
  builder.bind_label(test);
  builder.emit(Ret, 0..0);

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
  builder.emit_jump(&test, 0..0);

  let jump_len = 4; // jump instruction will use 4 bytes
  let num_nops = 65536 - jump_len; // fill the rest of 16bit offset with nops
  for _ in 0..num_nops {
    builder.emit(Nop, 0..0);
  }
  let jump_target = builder.bytecode.len() as u32;
  builder.bind_label(test);
  builder.emit(Ret, 0..0);

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
  builder.emit_jump(&test, 0..0);

  let jump_len = 6; // jump instruction will use 6 bytes
  let num_nops = 65536 - jump_len; // fill the rest of 16bit offset with nops
  for _ in 0..num_nops {
    builder.emit(Nop, 0..0);
  }
  let jump_target = builder.bytecode.len() as u32;
  builder.bind_label(test);
  builder.emit(Ret, 0..0);

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

#[test]
fn emit_jump_loop() {
  let mut builder = BytecodeBuilder::new();

  builder.emit(Nop, 0..0);
  let start = builder.loop_header();
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit(Nop, 0..0);
  builder.emit_jump_loop(&start, 0..0);
  builder.emit(Ret, 0..0);

  let (bytecode, constants) = builder.finish();

  assert!(constants.is_empty());
  assert_eq!(
    bytecode,
    [
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::Nop as u8,
      Opcode::JumpLoop as u8,
      /* offset */ (4 - 1),
      Opcode::Ret as u8,
    ]
  );

  assert_snapshot!(Disassembly::new(&bytecode, &constants, 0).to_string());
}

#[rustfmt::skip]
#[test]
fn emit_multi_label() {
  let mut builder = BytecodeBuilder::new();

  let labels = builder.multi_label("test", 4);
  for _ in 0..4 {
    builder.emit(Nop, 0..0);
    builder.emit_jump(labels.get(), 0..0);
  }
  builder.emit(Nop, 0..0);
  builder.bind_multi_label(labels);
  builder.emit(Ret, 0..0);

  let (bytecode, constants) = builder.finish();

  assert_eq!(
    bytecode,
    [
      Opcode::Nop as u8,
      Opcode::Jump as u8, /*offset*/ 12,
      Opcode::Nop as u8,
      Opcode::Jump as u8, /*offset*/ 9,
      Opcode::Nop as u8,
      Opcode::Jump as u8, /*offset*/ 6,
      Opcode::Nop as u8,
      Opcode::Jump as u8, /*offset*/ 3,
      Opcode::Nop as u8,
      Opcode::Ret as u8,
    ]
  );

  assert_snapshot!(Disassembly::new(&bytecode, &constants, 0).to_string());
}
