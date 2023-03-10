pub mod emitter;
pub mod error;
pub mod regalloc;

pub use emitter::emit;
pub use error::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;
