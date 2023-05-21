// TODO: get rid of this somehow
// it's only used in `value/object.rs` to access the vtable of a `dyn Object`

#[macro_use]
pub mod macros;

#[macro_use]
mod util;

#[macro_use]
mod object;

mod bytecode;
mod emit;
mod error;
#[cfg(feature = "serde")]
mod serde;
pub mod span;
mod syntax;
mod value;
mod vm;

pub mod public;
pub use public::*;

/*
- [x] remove `__miri` feature and put args in `xtask miri` instead (`--filter` for running only specific tests)
  - [x] should unlock `--all-features`
- [x] change ptr repr to use manual vtable + static object trait
- [x] remove Table `named_field`, it should be reserved for methods
- [ ] move all field access/index access/etc. to delegate to the object trait
- [ ] vm fully async
  - [x] stackless import
  - [ ] stackless class init
    - [ ] make `init` a contextual keyword that doesn't require `fn` in classes
    - [ ] user cannot access initializer (to call it again)
          only way is to call the class type
    - [ ] remove `super.init()` and replace with calling the proxy (`super()`)
- [ ] fix `from m import a, b, c` bug
- [ ] rename internal `hebi::String` to `hebi::Str`
- [ ] remove `Ref` from name of public value types (inner should be prefix by `Owned` or qualified path)
- [ ] fix `scope.params` will panic if given the wrong number of args
- [ ] change `This` to support any value (?)

- [ ] for iter loops
- [ ] generators
- [ ] list indexing
- [ ] f-strings
- [ ] ops on builtins
- [ ] methods on builtins
- [ ] timeout feature (abort signal, Arc<AtomicBool> or similar)
- [ ] semicolons (`;` for stmt, `;;` for block)
- [ ] report code locations (intern spans, track in hashmap of `offset->span_id`)
*/
