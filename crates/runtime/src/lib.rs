pub mod isolate;
mod util;
pub mod value;

/*
TODO: some cleanup after the vm+value merge

- `util`
- `error`


TODO: carefully design the public API
- Value
  - constructors
  - as_*
- Isolate
  - call
  - ?
*/

pub use isolate::Isolate;
pub use value::object::Error;
pub use value::Value;

pub type Result<T, E = Error> = std::result::Result<T, E>;
