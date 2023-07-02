use super::symbolic::*;
use super::*;
use crate::internal::bytecode::builder::BytecodeBuilder;

#[rustfmt::skip]
#[test]
fn register_patching() {
  let mut builder = BytecodeBuilder::new();

  builder.emit(Load {
    reg: Register(0) 
  }, 0..0);
  builder.emit(Load {
    reg: Register(1) 
  }, 0..0);
  builder.emit(Load {
    reg: Register(256) 
  }, 0..0);
  builder.emit(Load {
    reg: Register(65536) 
  }, 0..0);
  builder.emit(MakeDataClass {
    desc: Constant(0),
    parts: Register(0),
  }, 0..0);
  builder.emit(MakeDataClass {
    desc: Constant(0),
    parts: Register(1),
  }, 0..0);
  builder.emit(MakeDataClass {
    desc: Constant(0),
    parts: Register(256),
  }, 0..0);
  builder.emit(MakeDataClass {
    desc: Constant(0),
    parts: Register(65536),
  }, 0..0);

  let map = vec![127usize; 65537];

  let (mut bytecode, _) = builder.finish();

  assert_eq!(
    bytecode,
    [
      Opcode::Load as u8, /*register*/ 0,
      Opcode::Load as u8, /*register*/ 1,
      Opcode::Wide16 as u8, Opcode::Load as u8, /*register*/ 0, 1,
      Opcode::Wide32 as u8, Opcode::Load as u8, /*register*/ 0, 0, 1, 0,
      Opcode::MakeDataClass as u8, /*path*/ 0, /*destination*/ 0,
      Opcode::MakeDataClass as u8, /*path*/ 0, /*destination*/ 1,
      Opcode::Wide16 as u8, Opcode::MakeDataClass as u8, /*path*/ 0, 0, /*destination*/ 0, 1,
      Opcode::Wide32 as u8, Opcode::MakeDataClass as u8, /*path*/ 0, 0, 0, 0, /*destination*/ 0, 0, 1, 0,
    ]
  );

  patch_registers(&mut bytecode, &map);

  assert_eq!(
    bytecode,
    [
      Opcode::Load as u8, /*register*/ 127,
      Opcode::Load as u8, /*register*/ 127,
      Opcode::Wide16 as u8, Opcode::Load as u8, /*register*/ 127, 0,
      Opcode::Wide32 as u8, Opcode::Load as u8, /*register*/ 127, 0, 0, 0,
      Opcode::MakeDataClass as u8, /*path*/ 0, /*destination*/ 127,
      Opcode::MakeDataClass as u8, /*path*/ 0, /*destination*/ 127,
      Opcode::Wide16 as u8, Opcode::MakeDataClass as u8, /*path*/ 0, 0, /*destination*/ 127, 0,
      Opcode::Wide32 as u8, Opcode::MakeDataClass as u8, /*path*/ 0, 0, 0, 0, /*destination*/ 127, 0, 0, 0,
    ]
  );
}
