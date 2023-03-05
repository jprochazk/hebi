# 0.2.0

Note: Accidentally skipped `0.1.0` by using `cargo workspaces publish` incorrectly.

- Implemented module loading, configurable via the `ModuleLoader` trait
- Changed import syntax to match Python:

```python
import module
from module import thing
from module import a, b
from module import a as this, b as that
# etc.
```

Note that relative imports are not yet implemented.

- Implemented native function binding

```rust
#[hebi::function]
fn my_fn(v: i32) -> String {
  format!("value: {v}")
}

let vm = Hebi::default();
vm.globals().set("my_fn", vm.create_function(my_fn));

let output = vm.eval::<String>(r#"my_fn(1000)"#).unwrap();
println!("{output}");
```

Supports using the following types in parameters:
- integer types, signed and unsigned, all sizes
- `f32`/`f64`
- `bool`
- `()`
- `String`
- `&str`
- Any of those types inside of an `Option`

On top of supporting positional arguments, the attribute also supports:

Default values:
```rust
#[hebi::function]
fn my_fn(#[default(100)] v: i32) -> String {
  format!("value: {v}")
}

vm.eval::<()>("print my_fn()").unwrap(); // prints `100`
vm.eval::<()>("print my_fn(321)").unwrap(); // prints `321`
```

An `Option<T>` parameter will be treated the same as one with a default value.

Keyword parameters:
```rust
#[hebi::function]
fn my_fn(#[kw] v: i32) -> String {
  format!("value: {v}")
}

vm.eval::<()>("print my_fn(v=321)").unwrap(); // prints `321`
```

And they may be mixed:
```rust
#[hebi::function]
fn my_fn(
  #[default(100)] a: i32,
  #[kw] #[default(200)] b: i32
) -> String {
  format!("{a} + {b} = {}", a + b)
}

vm.eval::<()>("print my_fn()").unwrap();           // prints `100 + 200 = 300`
vm.eval::<()>("print my_fn(200)").unwrap();        // prints `200 + 200 = 400`
vm.eval::<()>("print my_fn(b=300)").unwrap();      // prints `100 + 300 = 400`
vm.eval::<()>("print my_fn(400, b=300)").unwrap(); // prints `400 + 300 = 700`
```

The order of params must be `positional -> default positional -> keyword`.
This is the same order that parameters of functions declared within scripts must follow.
For example, the following is invalid:
```rust
#[hebi::function]
fn my_fn(#[kw] a: i32, b: i32) -> String {
  todo!()
}
```
The macro will emit an error in this case.

Lastly, to return errors from functions, use `Result` with the `Err` variant containing:

- `String`
- `Vec<syntax::Error>`
- `Box<dyn std::error::Error + 'static>`

Returning an error from a native function will cause the VM to panic and unwind its stack,
eventually yielding back to the top-level `eval` call with the same error.

- Reworked conversion traits (`FromHebi`/`FromHebiRef`,`IntoHebi`)
- Unified `Error` type used in the VM

# 0.0.1

Rebranded to `Hebi`, and released on [crates.io](https://crates.io/).

- Implemented `Debug` and `Display` for `EvalError`
