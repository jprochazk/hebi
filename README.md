<p align="center">
  <img
    alt="A snake inside of a gear shape"
    src="./assets/logo.svg"
    height="192px"
  >
</p>

# Hebi

This repository hosts a dynamically typed language, its compiler, and VM.

### Usage

Everything is still in flux, and many interesting features are not yet implemented (see [issues](https://github.com/jprochazk/hebi/issues)), but it can already execute some basic code:

```rust
use hebi::Hebi;

let hebi = Hebi::new();

// prints `2`
println!("{}", hebi.eval::<i32>("1 + 1").unwrap());

hebi.eval::<()>(
  r#"
class Test:
  v = 10
  fn test(self):
    print self.v

t := Test(v=100)
t.test() # prints 100
t.v = 20
t.test() # prints 20
"#,
)
.unwrap();
```

To see more examples, visit [src/tests](./src/tests). A general overview of the language's currently implemented syntax and semantics is available in the [design](./design.md) file.

The language also has a REPL at [examples/cli](./examples/cli):

```
$ cargo run --example cli
Hebi REPL v0.0.0
Press CTRL-D to exit
> 
```

### Development

All you need to contribute is a recent version of the Rust compiler. See [Getting Started](https://www.rust-lang.org/learn/get-started) for how to obtain it.

Other tooling that is highly recommended:
- [rust-analyzer](https://rust-analyzer.github.io/), a Rust language server for your editor of choice
- [clippy](https://github.com/rust-lang/rust-clippy), a helpful linter
- [just](https://github.com/casey/just), which is used to run various commands


### Repository structure

- [`src`](./src) - The core crate, containing the runtime (bytecode compiler, register allocator, value representation, virtual machine).
  - [`op`](./src/op) - Bytecode instruction definitions, which define the fine-grained operations that the virtual machine may perform.
  - [`isolate`](./src/isolate) - The virtual machine, which implements the operations defined by `op`.
  - [`value`](./src/value) - Hebi's Value implementation, which is how the virtual machine represents values at runtime.
  - [`emit`](./src/emit) - The bytecode compiler, which transforms an AST to executable bytecode.
  - [`tests`](./src/tests) - End-to-end tests of the full evaluation pipeline (`Code -> AST -> Bytecode -> Output`).
- [`crates`](./crates) - Parts of the compiler which may be useful for building other tools, such as formatters, linters, and language servers.
  - [`span`](./crates/span) - Span implementation. Spans are how the compiler represents locations in code.
  - [`syntax`](./crates/syntax) - The lexer, parser, and AST. This crate accepts some text input, and outputs a structured representation of valid code, or some syntax errors.
  - [`diag`](./crates/diag) - Diagnostic tools for reporting useful errors to users.
  - [`derive`](./crates/derive) - (WIP) Derive macros for easily exposing Rust functions and objects to the runtime.

### Goals, Design and Implementation

The main goal is to have a reasonably feature-full language with simple (as in, can fit in a single person's head), unsurprising semantics that's easily embeddable within a Rust program. It isn't a priority for it to be a standalone, general-purpose language, but it isn't out of the question.

The language design is heavily inspired by Python. Hebi blatantly copies most of Python's syntax, and a lot of the semantics, but some parts have been removed, or changed. Python is very difficult to optimize, and has many quirks which make it unsuitable for embedding within another program.

The implementation borrows a lot of ideas from [V8](https://v8.dev/). Ideas shamelessly stolen include:
- Bytecode encoded with variable-width operands using a prefix opcode.
  This means there is no limit to the number of variables or constants in a function, the maximum distance of a jump, etc. It also results in very compact bytecode.
- An implicit accumulator to store temporary values. 
  This greatly reduces call frame sizes and the number of register moves. It also helps make the bytecode more compact, because every instruction operating on two or more registers now needs one less register operand.
- Function calling convention which copies arguments into the new call frame, instead of referencing them directly.
  This opens up the implementation space for stackless coroutines.
- Module variables on top of globals.
  This allows for a clear separation of concerns into different modules, as functions declared in different modules may not access the variables in each others' global scopes, unless they are explicitly imported.

Currently, the VM uses reference counting as its garbage collection strategy, but the plan is to [implement a tracing garbage collector](https://github.com/jprochazk/hebi/issues/6) at some point. Some possible approaches are described in [LuaJIT's wiki](http://web.archive.org/web/20220524034527/http://wiki.luajit.org/New-Garbage-Collector).

### Why Hebi?

I thought it was appropritate, because the language is in the Python family, and Hebi (蛇) means snake in Japanese. 

### License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
