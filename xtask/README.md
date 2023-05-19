# xtask

This repository uses [cargo-xtask](https://github.com/matklad/cargo-xtask) for various utilities, scripts, and tasks.

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

## Adding tasks

To add a new task:

* Think of a good name
* Create a file for it under [`src/task`](./src/task/)
* Expose a `run` function from it
  * This is the entrypoint to your task
* In [`src/task.rs`](./src/task.rs):
  * Add it as a submodule
  * Add some help text for it to the `HELP` constant
  * Add a match arm for it in `run`

That's it. Your task should now be available as `cargo xtask <your-task>`.
