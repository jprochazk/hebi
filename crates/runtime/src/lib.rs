pub mod isolate;
mod util;
pub mod value;

/*
TODO: some cleanup after the vm+value merge

- `util`
- `error`

finish the object access API


TODO: carefully design the public API
- Value
  - constructors
  - as_*
- Isolate
  - call
  - ?
*/

pub use isolate::{Error, Isolate};
pub use value::Value;
