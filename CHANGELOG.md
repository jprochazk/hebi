# UNRELEASED

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

# 0.0.1

Rebranded to `Hebi`, and released on [crates.io](https://crates.io/).

- Implemented `Debug` and `Display` for `EvalError`
