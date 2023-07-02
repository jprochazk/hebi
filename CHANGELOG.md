# 0.4.0

This is a total rewrite of the VM and codegen, which means that the API is now completely different.
The easiest way to get started is to look at the [examples](./examples).

# 0.3.1

## New features

This release introduces methods on builtin types.

```rust
let vm = Hebi::new();

// prints `[0, 1, 2, 3]`
vm.eval::<()>(r#"
v := [0,1,2]
v.push(3)
print v
"#).unwrap();
```

This release only adds a single method on the builtin `List` type, `push`. The goal was to lay the groundwork for implementing a proper standard library in the future.

# 0.3.0

## Breaking changes

- `Hebi::create_function` has been removed. It has been replaced by the `Globals::register_fn` method.
- `Hebi::with_io` (deprecated since `0.2.0`) has been removed.

## New APIs

- `Hebi::wrap` and `Hebi::try_wrap`
- `Globals::register_fn` and `Globals::register_class`

## New features

This release introduces native class binding.

```rust
#[hebi::class]
struct Number {
  value: i32
}

#[hebi::methods]
impl Number {
  #[init]
  pub fn new(value: i32) -> Self {
    Self { value }
  }

  pub fn add(&mut self, value: i32) {
    self.value += value;
  }

  pub fn square(&self) -> i32 {
    self.value * self.value
  }
}

let vm = Hebi::new();
vm.globals().register_class::<Number>();

vm.eval::<()>(r#"

a := Number(100)
print a.value # prints `100`
a.add(10)
print a.value # prints `110`

"#).unwrap();
```

Native class methods also support the `#[kw]` and `#[default]` parameter attributes.

It is also possible to pass a value to the VM instead of constructing it in a script:

```rust
// the type must be registered first, otherwise `wrap` will panic
vm.globals().register_class::<Number>();
vm.globals().set("n", vm.wrap(Number { value: 50 }));
vm.eval::<()>(r#"print n.square()"#).unwrap();
```

The value will be managed by the runtime. If it is no longer in use, it may be dropped at some point. The only time the value is guaranteed to be dropped is when the `Hebi` instance is dropped.

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
