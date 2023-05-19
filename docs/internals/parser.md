## Parser

The syntax of Hebi is heavily inspired by Python. It's a simple syntax with few symbols and significant indentation. That last part comes at a cost, as the parser must track the current indentation level and emit errors if the indentation rules are broken. The lexer is auto-generated using [logos](https://github.com/maciejhirsz/logos), and the parser is a hand-written recursive descent parser.

## Tracking indentation

The method used to track indentation is to count the leading whitespace of every token, and ignore it if the token is not the first on its line.

For example, consider the two following lines:

```python
if true:
  print "Hello, world!"
```

The token stream produced by the lexer would be:

```
 Token("if", ws=0)
 Token("true", ws=None)
 Token(":", ws=None)
 Token("print", ws=2)
 Token("\"Hello, world!\"", ws=None)
```

This representation exists only for illustrative purposes. The `ws` parameter shows the leading whitespace of each token. Notice how there's a distinction between `0` and `None`, and that tokens which are not the first on their respective lines do not have any whitespace.

Because whitespace is not a separate token, and is instead attached to the non-whitespace tokens, the parser is actually free to completely ignore it, and it does by default. The current indentation level is only queried and updated at points in the syntax where it matters. That is, instead of thinking "this part of the syntax does not need to care about whitespace", you instead think "this part of the syntax *does* need to care about whitespace". The parser code is in return somewhat clearer, and definitely easier to maintain.



