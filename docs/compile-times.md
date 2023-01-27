Cargo reports decently accurate build times by just doing `cargo clean && cargo build --workspace --release`

- Disable incremental compilation and debug info in CI
- Use [cargo llvm-lines](https://github.com/dtolnay/cargo-llvm-lines) to find things to monomorphize [^1]
- Improve parallelization by splitting crates where it makes sense.
- Reduce dependencies by disabling default features and only choosing what you really need.

[^1]: To monomorphize a function, use the following pattern:

```rust
fn generic<T: AsRef<str>>(s: T) -> String {
  s.as_ref().into()
}
```
Transform the above into:
```rust
fn generic<T: AsRef<str>>(s: T) -> String {
  fn generic_inner(s: &str) -> String {
    String::from(s)
  }
  generic_inner(s.as_ref())
}
```
The outer function remains generic, but it now consists of just a single call to a non-generic function. The instantiation of `generic` is now tiny, which means it is also significantly faster to compile. It will ultimately be compiled away, so it doesn't have any overhead.
