<p align="center">
  <img
    alt="A snake inside of a gear shape"
    src="./assets/logo.svg"
    height="192px"
  >
</p>

# Hebi

This repository hosts a dynamically typed language, its compiler, and VM.

🚧 This branch contains a rewrite of the language. It is currently heavily WIP! 🚧

## Usage

Install the library (currently only available via git):

```
$ cargo add --git https://github.com/jprochazk/hebi.git --branch v2
```

Import it, and run some code:

```rust
use hebi::Hebi;

fn main() {
  let mut hebi = Hebi::new();

  println!("1 + 1 = {}", hebi.eval("1 + 1").unwrap());
}
```

Hebi can do much more than this, though! Here are some of its features:

- A simple syntax inspired by Python
- First-class functions
- Classes
- Single inheritance
- Easy Rust function and struct binding
- Async support

## Development

The first step is to install Rust and Cargo via [rustup](https://rustup.rs/).

Hebi currently uses at least one unstable feature ([`ptr_metadata`](https://github.com/rust-lang/rust/issues/81513)), which means a nightly toolchain is required.

The best way is to configure rustup to use the nightly toolchain only in this repository:
```
$ rustup override set nightly
```

Or if you prefer living on the edge, set it as the default:
```
$ rustup default nightly
```

The compile times should already be plenty fast, but you can gain :
- Setting up a faster linker

  On Linux, [`mold`](https://github.com/rui314/mold) can be quite a bit faster than either `gold` or `lld`.
  To use it, you'll have to [build it](https://github.com/rui314/mold#how-to-build), and then add the following
  to your `$HOME/.cargo/config.toml`:

  ```toml
  [target.x86_64-unknown-linux-gnu]
  linker = "clang"
  rustflags = ["-C", "link-arg=-fuse-ld=/path/to/mold"]
  ```

  Replacing `/path/to/mold` with the actual path to the `mold` executable.

- Using Cargo's sparse protocol
  
  Add the following to your `$HOME/.cargo/config.toml`:
  ```toml
  [registries.crates-io]
  protocol = "sparse"
  ```

### xtask

This repository uses [cargo-xtask](https://github.com/matklad/cargo-xtask) for various utilities, scripts, and tasks. That means you don't need anything other than Rust and Cargo. No makefiles, Python, or Bash.

To see all available tasks, run:
```
$ cargo xtask
```

To run one of them, use:
```
$ cargo xtask <task>
```

For example:
```
# Run all tests and examples
$ cargo xtask test-all
```

Some tasks use tools which you'll have to install, though these are kept to just a select few, and ideally always installed through either `rustup` or `cargo`.

- [Miri](https://github.com/rust-lang/miri) (`rustup +nightly component add miri`)
- [Insta](https://insta.rs/) (`cargo install --locked cargo-insta`)
- [mdBook](https://github.com/rust-lang/mdBook) (`cargo install --locked mdbook`)

## Why Hebi?

I thought it was appropritate, because the language is in the Python family, and Hebi (蛇) means snake in Japanese. 

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
