use super::*;
// Things to test:

// 1. bytecode emit + disassembly
// 2. `run` loop
//   - jumps
//   - suspend
//   - errors

// write snapshots for all of the above

type Builder = BytecodeBuilder<()>;

#[test]
fn test_builder() {
  let mut b = Builder::new("test");

  let [start, end] = b.labels(["start", "end"]);

  // TODO: fix jump patching by emitting patched instructions
  // into a new BytecodeArray
  // and maybe improve the output a bit:
  // - print bytes as hex, left-aligned, space between bytes, min width = 6 bytes
  //   in this format
  // - don't put space between op name and operands

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

  let mut pc = 0;
  while pc < chunk.bytecode.len() {
    let instr = chunk.bytecode.disassemble(pc).unwrap();
    pc += instr.size();
    println!("{instr}");
  }
}
