#![allow(
  clippy::just_underscores_and_digits,
  non_upper_case_globals,
  clippy::needless_range_loop
)]

// TODO: simplify this process,
// but not at the expense of the ability to debug the code
//
// Adding a new instruction:
// 1. Declare it using `instruction!(...)`
// 2. Add it to the `opcode::extra!` list
// 3. Add disassembly in opcode::disassembly::disassemble
// 4. Add a method for it to `builder::BytecodeBuilder`
// 5. Add a method for it to `handler::Handler`
// 6. Create a dispatch handler for it in `dispatch` using `dispatch_handler!`
// 7. Add it to the match in `dispatch::run`

pub mod builder;
pub mod chunk;
mod dispatch;
pub mod handler;
pub mod opcode;

pub mod prelude {
  pub use crate::builder::BytecodeBuilder;
  pub use crate::chunk::{BytecodeArray, Chunk};
  pub use crate::dispatch::run;
  pub use crate::handler::{Handler, Jump};
}

#[cfg(test)]
mod tests;
