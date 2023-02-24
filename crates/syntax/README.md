# Syntax

This crate holds the Hebi [lexer](./src/lexer.rs), [parser](./src/parser.rs), and [AST](./src/ast.rs).

The lexer is automatically generated using [logos](https://github.com/maciejhirsz/logos). The parser is a hand-written [recursive descent parser](https://en.wikipedia.org/wiki/Recursive_descent_parser).

Indentation is lexed by assigning the first non-whitespace token on each line the number of whitespace characters that precede it. For example:
```
asdf
  asdf
  asdf asdf
```
Would produce the following tokens:
```
Identifier("asdf", indentation_level=0)
Identifier("asdf", indentation_level=2)
Identifier("asdf", indentation_level=2)
Identifier("asdf", indentation_level=None)
```
Note the last token, which doesn't have any indentation, because it is not the first non-whitespace token on its line.

The parser uses the indentation levels to track blocks using [these functions](https://github.com/jprochazk/hebi/blob/60ebb2818944f503cb6b1fd811e6fa511ab43bea/crates/syntax/src/parser.rs#L122-L167):
- `no_indent`, no indentation may be attached to the current token
- `indent_eq`, the indentation level of the current token is equal to the current indentation stack
- `indent_gt`, the indentation level of the current token is greater than the current indentation stack.
  This function also adds the new indentation level to the indentation stack.
- `dedent`, the indentation level of the current token is lower than the current indentation stack.
  This functino also pops the last indentation level off of the indentation stack.

These functions are used to query for indentation at strategic places, but the parser code can be written without caring about the indentation where it doesn't matter. For example, see the [`import_stmt`](https://github.com/jprochazk/hebi/blob/60ebb2818944f503cb6b1fd811e6fa511ab43bea/crates/syntax/src/parser/stmt.rs#L33) node, which does not care about indentation at all, and so it doesn't have to track it, either!

