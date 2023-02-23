# Diag

This crate implements error reporting for Mu. In simple terms, it turns `code + span + message` into a nicely formatted error message:

```
error: mismatched type
> test.foo:1
| 
| let x: Foo = Bar {
|   a: 0,
|   ...
|   g: 0,
| };
| 
+ expected `Foo`, found `Bar`
```

It's actually completely agnostic to the target language, it operates on simple strings and spans.

There are many crates that implement something similar, for example:
- [codespan](https://github.com/brendanzab/codespan)
- [annotate-snippets](https://github.com/rust-lang/annotate-snippets-rs)
- [codemap](https://github.com/kevinmehall/codemap)
- [language-reporting](https://github.com/wycats/language-reporting)

And while they are all good at what they do, they all ended up having features I wouldn't use, and I wanted to keep the dependency graph as minimal as possible.
