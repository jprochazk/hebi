# Mu CLI

Usage:
```
Usage: cli [COMMAND]

Commands:
  repl  
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### `cli repl`
```
Usage: cli repl

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Notes

Implemented using the [`clap`](https://github.com/clap-rs/clap) crate for parsing arguments, and the [`rustyline`](https://github.com/kkawakam/rustyline) for the `Read` part of Read-Eval-Print-Loop.

TODO: use https://github.com/nushell/reedline instead
