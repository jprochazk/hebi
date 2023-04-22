// TODO: get rid of this somehow
// it's only used in `value/object.rs` to access the vtable of a `dyn Object`
#![feature(ptr_metadata)]

#[macro_use]
mod util;

pub mod ctx;
pub mod error;
pub mod op;
pub mod span;
pub mod syntax;
pub mod value;
pub mod vm;
