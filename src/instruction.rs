// TODO: bytecode stream
// encode instructions as
//   [opcode : u8] [operands : ...]
// instructions will have LONG variants for larger operands where necessary

// TODO: more `long` instructions which store operands as constants
// TODO: document the rest

pub mod builder;
pub mod opcodes;
pub mod operands;
