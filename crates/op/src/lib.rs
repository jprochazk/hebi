#![allow(
  clippy::just_underscores_and_digits,
  non_upper_case_globals,
  clippy::needless_range_loop
)]

// TODO: simplify this process,
// but not at the expense of the ability to debug the code
//
// Adding a new instruction:
// - in `mod opcode`:
//   - Declare it using `instruction!(...)`
//   - Add it to `extra!{...}` list
//   - in `mod disassembly`:
//     - Add a branch for it in `disassemble`
// 3. Add disassembly in opcode::disassembly::disassemble
// 4. Add a method for it to `handler::Handler`
// 5. Create a dispatch handler for it in `dispatch` using `dispatch_handler!`
// 6. Add it to the match in `dispatch::run`

pub mod builder;
pub mod chunk;
pub mod disassembly;
mod dispatch;
pub mod handler;
pub mod opcode;

pub mod prelude {
  pub use crate::builder::BytecodeBuilder;
  pub use crate::chunk::{BytecodeArray, Chunk};
  pub use crate::dispatch::run;
  pub use crate::handler::{ControlFlow, Handler};
  pub use crate::opcode::*;
}

#[cfg(test)]
mod tests;
